use std::collections::HashMap;
use std::hash::{DefaultHasher, Hash, Hasher};
use std::path::{Path, PathBuf};
use std::process::Child;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, mpsc};
use std::time::Duration;

use globset::{Glob, GlobSet, GlobSetBuilder};
use notify_debouncer_full::{
    DebounceEventResult, DebouncedEvent, new_debouncer,
    notify::{EventKind, RecursiveMode, Watcher, event::ModifyKind},
};
use walkdir::WalkDir;

use crate::config::Target;
use crate::runner;

const DEBOUNCE_MS: u64 = 100;

pub fn build_ignore_set(patterns: &[String]) -> GlobSet {
    let mut builder = GlobSetBuilder::new();
    for pat in patterns {
        let globs: &[String] = &if let Some(dir) = pat.strip_suffix('/') {
            vec![format!("**/{dir}"), format!("**/{dir}/**")]
        } else if pat.contains(['*', '?', '[']) {
            vec![pat.clone()]
        } else {
            vec![format!("**/{pat}"), format!("**/{pat}/**")]
        };
        for g in globs {
            if let Ok(glob) = Glob::new(g) {
                builder.add(glob);
            }
        }
    }
    builder.build().unwrap_or_else(|_| GlobSet::empty())
}

fn has_glob(s: &str) -> bool {
    s.contains(['*', '?', '['])
}

fn glob_base(pattern: &str) -> PathBuf {
    let base: PathBuf = pattern
        .split('/')
        .take_while(|seg| !seg.contains(['*', '?', '[']))
        .collect();
    if base.as_os_str().is_empty() {
        PathBuf::from(".")
    } else {
        base
    }
}

pub fn watch_config(path: &Path, stop: &AtomicBool, config_changed: &AtomicBool) {
    let (tx, rx) = mpsc::channel::<DebounceEventResult>();
    let Ok(mut debouncer) = new_debouncer(Duration::from_millis(DEBOUNCE_MS), None, tx) else {
        return;
    };
    if debouncer
        .watcher()
        .watch(path, RecursiveMode::NonRecursive)
        .is_err()
    {
        return;
    }
    let mut last_hash = hash_file(path);
    loop {
        if stop.load(Ordering::Relaxed) {
            return;
        }
        match rx.recv_timeout(Duration::from_millis(100)) {
            Ok(_) => {
                let new_hash = hash_file(path);
                if new_hash != last_hash {
                    config_changed.store(true, Ordering::Relaxed);
                    stop.store(true, Ordering::Relaxed);
                    return;
                }
                last_hash = new_hash;
            }
            Err(mpsc::RecvTimeoutError::Timeout) => continue,
            Err(mpsc::RecvTimeoutError::Disconnected) => return,
        }
    }
}

pub fn watch(
    name: &str,
    target: &Target,
    ignore: &GlobSet,
    stop: Arc<AtomicBool>,
) -> Result<(), String> {
    let (tx, rx) = mpsc::channel::<DebounceEventResult>();

    let mut debouncer = new_debouncer(Duration::from_millis(DEBOUNCE_MS), None, tx)
        .map_err(|e| format!("[{name}] failed to create watcher: {e}"))?;

    let cwd = std::env::current_dir().unwrap_or_default();

    let (glob_strs, plain_strs): (Vec<&String>, Vec<&String>) =
        target.watch.iter().partition(|p| has_glob(p));

    let mut plain_roots: Vec<PathBuf> = Vec::new();
    for s in &plain_strs {
        let path = PathBuf::from(s);
        debouncer
            .watcher()
            .watch(&path, RecursiveMode::NonRecursive)
            .map_err(|e| format!("[{name}] failed to watch `{}`: {e}", path.display()))?;
        debouncer
            .cache()
            .add_root(&path, RecursiveMode::NonRecursive);
        plain_roots.push(path);
    }

    let mut glob_bases: Vec<PathBuf> = Vec::new();
    for s in &glob_strs {
        let base = glob_base(s);
        if !glob_bases.contains(&base) {
            debouncer
                .watcher()
                .watch(&base, RecursiveMode::Recursive)
                .map_err(|e| format!("[{name}] failed to watch `{}`: {e}", base.display()))?;
            debouncer.cache().add_root(&base, RecursiveMode::Recursive);
            glob_bases.push(base);
        }
    }

    // absolutize glob patterns so they match the absolute paths notify emits
    let event_filter: Option<GlobSet> = if glob_strs.is_empty() {
        None
    } else {
        let mut builder = GlobSetBuilder::new();
        for s in &glob_strs {
            let abs = format!("{}/{s}", cwd.display());
            if let Ok(g) = Glob::new(&abs) {
                builder.add(g);
            }
        }
        for path in &plain_roots {
            let abs = cwd.join(path);
            if let Ok(g) = Glob::new(&format!("{}/*", abs.display())) {
                builder.add(g);
            }
            if let Ok(g) = Glob::new(&abs.to_string_lossy()) {
                builder.add(g);
            }
        }
        builder.build().ok()
    };

    println!(
        "[{name}] watching {} path(s){}",
        target.watch.len(),
        if target.interrupt { " (interrupt)" } else { "" }
    );

    let mut hashes = seed_hashes(&plain_roots, &glob_bases, event_filter.as_ref(), ignore);

    if target.interrupt {
        watch_interrupt(
            name,
            target,
            ignore,
            event_filter.as_ref(),
            rx,
            &mut hashes,
            &stop,
        )
    } else {
        watch_loop(
            name,
            ignore,
            event_filter.as_ref(),
            rx,
            &mut hashes,
            &stop,
            |triggered| {
                println!("\n[{name}] {}", triggered.label());
                dispatch(name, target, triggered);
            },
        )
    }
}

fn watch_loop(
    name: &str,
    ignore: &GlobSet,
    event_filter: Option<&GlobSet>,
    rx: mpsc::Receiver<DebounceEventResult>,
    hashes: &mut HashMap<PathBuf, u64>,
    stop: &AtomicBool,
    mut on_event: impl FnMut(&Event),
) -> Result<(), String> {
    loop {
        if stop.load(Ordering::Relaxed) {
            return Ok(());
        }
        match rx.recv_timeout(Duration::from_millis(DEBOUNCE_MS)) {
            Ok(result) => {
                let events = unwrap_or_warn(name, result);
                if let Some(triggered) = classify(&events, ignore, event_filter, hashes) {
                    on_event(&triggered);
                }
            }
            Err(mpsc::RecvTimeoutError::Timeout) => {}
            Err(mpsc::RecvTimeoutError::Disconnected) => return Ok(()),
        }
    }
}

// we interrupt currently running processes if their flag is set
fn watch_interrupt(
    name: &str,
    target: &Target,
    ignore: &GlobSet,
    event_filter: Option<&GlobSet>,
    rx: mpsc::Receiver<DebounceEventResult>,
    hashes: &mut HashMap<PathBuf, u64>,
    stop: &AtomicBool,
) -> Result<(), String> {
    let mut current: Option<Child> = None;
    watch_loop(name, ignore, event_filter, rx, hashes, stop, |triggered| {
        if let Some(mut child) = current.take() {
            eprint!("[{name}] interrupting... ");
            runner::kill(&mut child);
        }
        println!("[{name}] {}", triggered.label());
        if !target.run.is_empty() {
            match runner::spawn(&target.run, name) {
                Ok(child) => current = Some(child),
                Err(e) => eprintln!("[{name}] {e}"),
            }
        }
        run_specific(name, target, triggered);
    })?;
    if let Some(mut child) = current {
        runner::kill(&mut child);
    }
    Ok(())
}

fn dispatch(name: &str, target: &Target, triggered: &Event) {
    if !target.run.is_empty() {
        if let Err(e) = runner::run(&target.run, name) {
            eprintln!("[{name}] {e}");
        }
    }
    run_specific(name, target, triggered);
}

fn run_specific(name: &str, target: &Target, triggered: &Event) {
    let handlers: [(&[String], bool, &str); 4] = [
        (&target.on_change, triggered.change, "onChange"),
        (&target.on_create, triggered.create, "onCreate"),
        (&target.on_delete, triggered.delete, "onDelete"),
        (&target.on_rename, triggered.rename, "onRename"),
    ];
    for (cmds, fired, label) in handlers {
        if fired && !cmds.is_empty() {
            if let Err(e) = runner::run(cmds, &format!("{name} {label}")) {
                eprintln!("[{name}] {e}");
            }
        }
    }
}

struct Event {
    change: bool,
    create: bool,
    delete: bool,
    rename: bool,
}

impl Event {
    fn label(&self) -> &'static str {
        match (self.change, self.create, self.delete, self.rename) {
            (true, false, false, false) => "modified",
            (false, true, false, false) => "created",
            (false, false, true, false) => "deleted",
            (false, false, false, true) => "renamed",
            _ => "changed",
        }
    }
}

fn classify(
    events: &[DebouncedEvent],
    ignore: &GlobSet,
    event_filter: Option<&GlobSet>,
    hashes: &mut HashMap<PathBuf, u64>,
) -> Option<Event> {
    let mut t = Event {
        change: false,
        create: false,
        delete: false,
        rename: false,
    };

    for event in events {
        if !event.paths.is_empty() && event.paths.iter().all(|p| ignore.is_match(p)) {
            continue;
        }
        if let Some(filter) = event_filter {
            if !event.paths.iter().any(|p| filter.is_match(p)) {
                continue;
            }
        }
        match &event.kind {
            EventKind::Modify(ModifyKind::Name(_)) => t.rename = true,
            EventKind::Modify(ModifyKind::Data(_) | ModifyKind::Any) => {
                for path in &event.paths {
                    // directories fire modify events when files inside them
                    // change - skip them, the file itself will have its own event.
                    if path.is_dir() {
                        continue;
                    }
                    if content_changed(path, hashes) {
                        t.change = true;
                    }
                }
            }
            EventKind::Create(_) => t.create = true,
            EventKind::Remove(_) => {
                for path in &event.paths {
                    hashes.remove(path);
                }
                t.delete = true;
            }
            _ => {}
        }
    }

    (t.change || t.create || t.delete || t.rename).then_some(t)
}

fn hash_dir(
    root: &Path,
    max_depth: Option<usize>,
    path_filter: Option<&GlobSet>,
    ignore: &GlobSet,
    hashes: &mut HashMap<PathBuf, u64>,
) {
    let cwd = std::env::current_dir().unwrap_or_default();
    let walker = WalkDir::new(root);
    let walker = if let Some(d) = max_depth {
        walker.max_depth(d)
    } else {
        walker
    };
    for entry in walker
        .into_iter()
        .filter_entry(|e| !ignore.is_match(e.path()))
        .flatten()
    {
        if entry.file_type().is_file() {
            let raw = entry.path();
            // Absolutize without canonicalizing — keeps the same format that notify
            // uses when delivering event paths, so the filter matches correctly.
            let abs = if raw.is_absolute() {
                raw.to_path_buf()
            } else {
                cwd.join(raw)
            };
            if path_filter.map_or(true, |f| f.is_match(&abs)) {
                if let Ok(canonical) = raw.canonicalize() {
                    if let Some(hash) = hash_file(&canonical) {
                        hashes.insert(canonical, hash);
                    }
                }
            }
        }
    }
}

fn seed_hashes(
    plain_roots: &[PathBuf],
    glob_bases: &[PathBuf],
    event_filter: Option<&GlobSet>,
    ignore: &GlobSet,
) -> HashMap<PathBuf, u64> {
    let mut hashes = HashMap::new();
    for root in plain_roots {
        hash_dir(root, Some(1), None, ignore, &mut hashes);
    }
    for root in glob_bases {
        hash_dir(root, None, event_filter, ignore, &mut hashes);
    }
    hashes
}

fn content_changed(path: &Path, hashes: &mut HashMap<PathBuf, u64>) -> bool {
    let key = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
    let Some(new_hash) = hash_file(path) else {
        hashes.remove(&key);
        return true;
    };
    let old = hashes.insert(key, new_hash);
    old != Some(new_hash)
}

fn hash_file(path: &Path) -> Option<u64> {
    let bytes = std::fs::read(path).ok()?;
    let mut h = DefaultHasher::new();
    bytes.hash(&mut h);
    Some(h.finish())
}

fn unwrap_or_warn(
    name: &str,
    result: Result<Vec<DebouncedEvent>, Vec<notify_debouncer_full::notify::Error>>,
) -> Vec<DebouncedEvent> {
    match result {
        Ok(events) => events,
        Err(errors) => {
            for e in errors {
                eprintln!("[{name}] watch error: {e}");
            }
            vec![]
        }
    }
}
