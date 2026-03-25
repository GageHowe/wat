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

pub fn watch(name: &str, target: &Target, ignore: &GlobSet) -> Result<(), String> {
    let (tx, rx) = mpsc::channel::<DebounceEventResult>();

    let mut debouncer = new_debouncer(Duration::from_millis(DEBOUNCE_MS), None, tx)
        .map_err(|e| format!("[{name}] failed to create watcher: {e}"))?;

    for path in target.watch_paths() {
        debouncer
            .watcher()
            .watch(&path, RecursiveMode::Recursive)
            .map_err(|e| format!("[{name}] failed to watch `{}`: {e}", path.display()))?;
        debouncer.cache().add_root(&path, RecursiveMode::Recursive);
    }

    println!(
        "[{name}] watching {} path(s){}",
        target.watch.len(),
        if target.interrupt { " (interrupt)" } else { "" }
    );

    // Seed the hash cache from disk so the very first save after startup
    // doesn't spuriously trigger if content hasn't changed.
    let mut hashes = seed_hashes(&target.watch_paths(), ignore);

    if target.interrupt {
        watch_interrupt(name, target, ignore, rx, &mut hashes)
    } else {
        watch_sequential(name, target, ignore, rx, &mut hashes)
    }
}

// ── Watch loops ──────────────────────────────────────────────────────────────

fn watch_sequential(
    name: &str,
    target: &Target,
    ignore: &GlobSet,
    rx: mpsc::Receiver<DebounceEventResult>,
    hashes: &mut HashMap<PathBuf, u64>,
) -> Result<(), String> {
    for result in rx {
        let events = unwrap_or_warn(name, result);
        let Some(triggered) = classify(&events, ignore, hashes) else {
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
    rx: mpsc::Receiver<DebounceEventResult>,
    hashes: &mut HashMap<PathBuf, u64>,
) -> Result<(), String> {
    let mut current: Option<Child> = None;

    for result in rx {
        let events = unwrap_or_warn(name, result);
        let Some(triggered) = classify(&events, ignore, hashes) else {
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
    hashes: &mut HashMap<PathBuf, u64>,
) -> Option<Triggered> {
    let mut t = Triggered { change: false, create: false, delete: false, rename: false };

    for event in events {
        if !event.paths.is_empty() && event.paths.iter().all(|p| ignore.is_match(p)) {
            continue;
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

/// Walk watched paths at startup and hash every file so the first save
/// after launch doesn't fire if nothing actually changed.
fn seed_hashes(watch_paths: &[PathBuf], ignore: &GlobSet) -> HashMap<PathBuf, u64> {
    let mut hashes = HashMap::new();
    for root in watch_paths {
        for entry in WalkDir::new(root)
            .into_iter()
            .filter_entry(|e| !ignore.is_match(e.path()))
            .flatten()
        {
            let path = entry.path();
            if path.is_file() {
                if let Some(hash) = hash_file(path) {
                    hashes.insert(path.to_path_buf(), hash);
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
