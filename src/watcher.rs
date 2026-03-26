use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::process::Child;
use std::sync::mpsc;
use std::time::Duration;

use globset::{Glob, GlobSet, GlobSetBuilder};
use notify_debouncer_mini::{
    DebounceEventResult, DebouncedEvent, new_debouncer,
    notify::RecursiveMode,
};

use crate::config::Target;
use crate::runner;

const DEBOUNCE_MS: u64 = 100;

struct CompiledTarget<'a> {
    name: &'a str,
    target: &'a Target,
    filter: GlobSet,
}

pub fn watch(targets: &[(&str, &Target)], ignore: &[String]) -> Result<(), String> {
    let cwd = std::env::current_dir().map_err(|e| format!("failed to read cwd: {e}"))?;
    let compiled = compile_targets(targets, &cwd)?;
    let ignore = build_ignore_set(ignore);
    let (tx, rx) = mpsc::channel::<DebounceEventResult>();
    let mut debouncer = new_debouncer(Duration::from_millis(DEBOUNCE_MS), tx)
        .map_err(|e| format!("failed to create watcher: {e}"))?;

    for root in watch_roots(targets) {
        let mode = if root.is_file() {
            RecursiveMode::NonRecursive
        } else {
            RecursiveMode::Recursive
        };
        debouncer
            .watcher()
            .watch(&root, mode)
            .map_err(|e| format!("failed to watch `{}`: {e}", root.display()))?;
    }

    for target in &compiled {
        println!(
            "[{}] watching {} path(s){}",
            target.name,
            target.target.watch.len(),
            if target.target.interrupt { " (interrupt)" } else { "" }
        );
    }

    let mut running: Vec<Option<Child>> = compiled
        .iter()
        .map(|target| {
            if !target.target.interrupt {
                return None;
            }
            match runner::start(&target.target.run, target.name) {
                Ok(child) => Some(child),
                Err(e) => {
                    eprintln!("[{}] {e}", target.name);
                    None
                }
            }
        })
        .collect();
    loop {
        match rx.recv() {
            Ok(Ok(events)) => {
                let matched = match_targets(&compiled, &ignore, &events);
                for index in matched {
                    trigger(&compiled[index], &mut running[index]);
                }
            }
            Ok(Err(error)) => eprintln!("[wat] watch error: {error}"),
            Err(_) => break,
        }
    }

    for child in &mut running {
        if let Some(child) = child {
            runner::kill(child);
        }
    }

    Ok(())
}

fn trigger(target: &CompiledTarget<'_>, child: &mut Option<Child>) {
    println!("\n[{}] changed", target.name);
    if target.target.interrupt {
        if let Some(running) = child {
            runner::kill(running);
        }
        match runner::start(&target.target.run, target.name) {
            Ok(next) => *child = Some(next),
            Err(e) => {
                *child = None;
                eprintln!("[{}] {e}", target.name);
            }
        }
        return;
    }

    if let Err(e) = runner::run(&target.target.run, target.name) {
        eprintln!("[{}] {e}", target.name);
    }
}

fn match_targets(
    targets: &[CompiledTarget<'_>],
    ignore: &GlobSet,
    events: &[DebouncedEvent],
) -> Vec<usize> {
    let mut matched = HashSet::new();
    for event in events {
        if ignore.is_match(&event.path) {
            continue;
        }
        for (index, target) in targets.iter().enumerate() {
            if target.filter.is_match(&event.path) {
                matched.insert(index);
            }
        }
    }
    let mut matched: Vec<_> = matched.into_iter().collect();
    matched.sort_unstable();
    matched
}

fn compile_targets<'a>(
    targets: &[(&'a str, &'a Target)],
    cwd: &Path,
) -> Result<Vec<CompiledTarget<'a>>, String> {
    targets
        .iter()
        .map(|&(name, target)| {
            let filter = build_watch_set(&target.watch, cwd)?;
            Ok(CompiledTarget { name, target, filter })
        })
        .collect()
}

fn watch_roots(targets: &[(&str, &Target)]) -> Vec<PathBuf> {
    let mut roots = Vec::new();
    for (_, target) in targets {
        for pattern in &target.watch {
            let root = watch_root(pattern);
            if !roots.contains(&root) {
                roots.push(root);
            }
        }
    }
    roots
}

fn watch_root(pattern: &str) -> PathBuf {
    if has_glob(pattern) {
        return first_existing_ancestor(glob_base(pattern));
    }
    first_existing_ancestor(PathBuf::from(pattern))
}

fn first_existing_ancestor(path: PathBuf) -> PathBuf {
    let mut current = if path.as_os_str().is_empty() {
        PathBuf::from(".")
    } else {
        path
    };
    while !current.exists() {
        if !current.pop() {
            return PathBuf::from(".");
        }
    }
    current
}

fn build_watch_set(patterns: &[String], cwd: &Path) -> Result<GlobSet, String> {
    let mut builder = GlobSetBuilder::new();
    for pattern in patterns {
        for glob in expand_watch_pattern(pattern, cwd) {
            let parsed = Glob::new(&glob)
                .map_err(|e| format!("invalid watch pattern `{pattern}`: {e}"))?;
            builder.add(parsed);
        }
    }
    builder.build().map_err(|e| format!("invalid watch set: {e}"))
}

fn build_ignore_set(patterns: &[String]) -> GlobSet {
    let mut builder = GlobSetBuilder::new();
    for pattern in patterns {
        for glob in expand_ignore_pattern(pattern) {
            if let Ok(parsed) = Glob::new(&glob) {
                builder.add(parsed);
            }
        }
    }
    builder.build().unwrap_or_else(|_| GlobSet::empty())
}

fn expand_watch_pattern(pattern: &str, cwd: &Path) -> Vec<String> {
    let abs = normalize(cwd.join(pattern));
    if has_glob(pattern) {
        return vec![as_glob(abs)];
    }
    let path = as_glob(abs);
    vec![path.clone(), format!("{path}/**")]
}

fn expand_ignore_pattern(pattern: &str) -> Vec<String> {
    if let Some(dir) = pattern.strip_suffix('/') {
        return vec![format!("**/{dir}"), format!("**/{dir}/**")];
    }
    if has_glob(pattern) {
        return vec![pattern.to_string()];
    }
    vec![format!("**/{pattern}"), format!("**/{pattern}/**")]
}

fn glob_base(pattern: &str) -> PathBuf {
    let base: PathBuf = pattern
        .split(['/', '\\'])
        .take_while(|segment| !has_glob(segment))
        .collect();
    if base.as_os_str().is_empty() {
        PathBuf::from(".")
    } else {
        base
    }
}

fn normalize(path: PathBuf) -> PathBuf {
    std::path::absolute(path).unwrap_or_else(|_| PathBuf::from("."))
}

fn as_glob(path: PathBuf) -> String {
    path.to_string_lossy().replace('\\', "/")
}

fn has_glob(value: &str) -> bool {
    value.contains(['*', '?', '['])
}
