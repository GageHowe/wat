use std::collections::HashMap;
use std::hash::{DefaultHasher, Hash, Hasher};
use std::path::{Path, PathBuf};
use std::process::Child;
use std::sync::mpsc;
use std::time::Duration;

use globset::{Glob, GlobSet, GlobSetBuilder};
use notify_debouncer_full::{
    new_debouncer,
    notify::{
        event::ModifyKind,
        EventKind, RecursiveMode, Watcher,
    },
    DebouncedEvent, DebounceEventResult,
};
use walkdir::WalkDir;

use crate::config::Target;
use crate::runner;

/// Debounce window — collapses editor save-storms while staying snappy.
const DEBOUNCE_MS: u64 = 100;

/// Compile ignore patterns into a `GlobSet`.
///
/// Patterns ending with `/` (e.g. `"target/"`) match the named directory and
/// everything inside it. Plain patterns (e.g. `".env"`) match any file or
/// directory with that name anywhere in the tree. Standard glob syntax
/// (`*.log`, `**/__pycache__`) is also accepted.
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

/// Returns the non-glob prefix of a glob pattern as a `PathBuf`.
/// `"src/**/*.rs"` → `"src"`, `"*.rs"` → `"."`.
fn glob_base(pattern: &str) -> PathBuf {
    let base: PathBuf = pattern
        .split('/')
        .take_while(|seg| !seg.contains(['*', '?', '[']))
        .collect();
    if base.as_os_str().is_empty() { PathBuf::from(".") } else { base }
}

pub fn watch(name: &str, target: &Target, ignore: &GlobSet) -> Result<(), String> {
    let (tx, rx) = mpsc::channel::<DebounceEventResult>();

    let mut debouncer = new_debouncer(Duration::from_millis(DEBOUNCE_MS), None, tx)
        .map_err(|e| format!("[{name}] failed to create watcher: {e}"))?;

    let cwd = std::env::current_dir().unwrap_or_default();

    let (glob_strs, plain_strs): (Vec<&String>, Vec<&String>) =
        target.watch.iter().partition(|p| has_glob(p));

    // Plain paths: watch the surface level only.
    let mut plain_roots: Vec<PathBuf> = Vec::new();
    for s in &plain_strs {
        let path = PathBuf::from(s);
        debouncer
            .watcher()
            .watch(&path, RecursiveMode::NonRecursive)
            .map_err(|e| format!("[{name}] failed to watch `{}`: {e}", path.display()))?;
        debouncer.cache().add_root(&path, RecursiveMode::NonRecursive);
        plain_roots.push(path);
    }

    // Glob paths: watch the base directory recursively; filter events by pattern.
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

    // Build an event filter for glob-watched paths.
    // Patterns are absolutized so they match the absolute paths notify emits.
    // When both plain and glob paths are present, plain paths are also added
    // to the filter so their events are not dropped.
    let event_filter: Option<GlobSet> = if glob_strs.is_empty() {
        None // plain+NonRecursive already scopes depth; no filter needed
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
            // Accept any file directly inside the plain directory.
            if let Ok(g) = Glob::new(&format!("{}/*", abs.display())) {
                builder.add(g);
            }
            // Also accept the path itself in case it's a single-file watch.
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

    // Seed the hash cache from disk so the very first save after startup
    // doesn't spuriously trigger if content hasn't changed.
    let mut hashes = seed_hashes(&plain_roots, &glob_bases, event_filter.as_ref(), ignore);

    if target.interrupt {
        watch_interrupt(name, target, ignore, event_filter.as_ref(), rx, &mut hashes)
    } else {
        watch_sequential(name, target, ignore, event_filter.as_ref(), rx, &mut hashes)
    }
}

// ── Watch loops ──────────────────────────────────────────────────────────────

fn watch_sequential(
    name: &str,
    target: &Target,
    ignore: &GlobSet,
    event_filter: Option<&GlobSet>,
    rx: mpsc::Receiver<DebounceEventResult>,
    hashes: &mut HashMap<PathBuf, u64>,
) -> Result<(), String> {
    for result in rx {
        let events = unwrap_or_warn(name, result);
        let Some(triggered) = classify(&events, ignore, event_filter, hashes) else {
            continue;
        };
        println!("\n[{name}] {}", triggered.label());
        dispatch(name, target, &triggered);
    }
    Ok(())
}

fn watch_interrupt(
    name: &str,
    target: &Target,
    ignore: &GlobSet,
    event_filter: Option<&GlobSet>,
    rx: mpsc::Receiver<DebounceEventResult>,
    hashes: &mut HashMap<PathBuf, u64>,
) -> Result<(), String> {
    let mut current: Option<Child> = None;

    for result in rx {
        let events = unwrap_or_warn(name, result);
        let Some(triggered) = classify(&events, ignore, event_filter, hashes) else {
            continue;
        };

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
        run_specific(name, target, &triggered);
    }

    if let Some(mut child) = current {
        runner::kill(&mut child);
    }
    Ok(())
}

// ── Dispatch ─────────────────────────────────────────────────────────────────

fn dispatch(name: &str, target: &Target, triggered: &Triggered) {
    if !target.run.is_empty() {
        if let Err(e) = runner::run(&target.run, name) {
            eprintln!("[{name}] {e}");
        }
    }
    run_specific(name, target, triggered);
}

fn run_specific(name: &str, target: &Target, triggered: &Triggered) {
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

// ── Event classification ─────────────────────────────────────────────────────

struct Triggered {
    change: bool,
    create: bool,
    delete: bool,
    rename: bool,
}

impl Triggered {
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
) -> Option<Triggered> {
    let mut t = Triggered { change: false, create: false, delete: false, rename: false };

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
                    // Directories fire modify events when files inside them
                    // change — skip them, the file itself will have its own event.
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

// ── Content hashing ──────────────────────────────────────────────────────────

/// Walk watched paths at startup and hash every relevant file so the first
/// save after launch doesn't fire if nothing actually changed.
fn seed_hashes(
    plain_roots: &[PathBuf],
    glob_bases: &[PathBuf],
    event_filter: Option<&GlobSet>,
    ignore: &GlobSet,
) -> HashMap<PathBuf, u64> {
    let mut hashes = HashMap::new();

    // Plain paths: surface-level only, matching the NonRecursive watch.
    for root in plain_roots {
        for entry in WalkDir::new(root)
            .max_depth(1)
            .into_iter()
            .filter_entry(|e| !ignore.is_match(e.path()))
            .flatten()
        {
            if entry.file_type().is_file() {
                if let Ok(path) = entry.path().canonicalize() {
                    if let Some(hash) = hash_file(&path) {
                        hashes.insert(path, hash);
                    }
                }
            }
        }
    }

    // Glob paths: recursive under the base, filtered by the event filter.
    for root in glob_bases {
        for entry in WalkDir::new(root)
            .into_iter()
            .filter_entry(|e| !ignore.is_match(e.path()))
            .flatten()
        {
            if entry.file_type().is_file() {
                if let Ok(path) = entry.path().canonicalize() {
                    if event_filter.map_or(true, |f| f.is_match(&path)) {
                        if let Some(hash) = hash_file(&path) {
                            hashes.insert(path, hash);
                        }
                    }
                }
            }
        }
    }

    hashes
}

/// Returns true if the file's content hash differs from the cache,
/// and updates the cache entry.
fn content_changed(path: &Path, hashes: &mut HashMap<PathBuf, u64>) -> bool {
    let Some(new_hash) = hash_file(path) else {
        // Unreadable — remove stale entry and treat as changed.
        hashes.remove(path);
        return true;
    };
    let old = hashes.insert(path.to_path_buf(), new_hash);
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
