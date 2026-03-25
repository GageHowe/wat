mod cli;
mod config;
mod runner;
mod watcher;

use std::process::ExitCode;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("error: {e}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<(), String> {
    let cli = match cli::parse()? {
        cli::Outcome::Help => {
            println!("{}", cli::help_text());
            return Ok(());
        }
        cli::Outcome::Run(c) => c,
    };

    let config_path = cli
        .config_path
        .clone()
        .map(Ok)
        .unwrap_or_else(config::find_config)?;

    loop {
        let config = config::load(&config_path)?;

        let names: Vec<&str> = if !cli.targets.is_empty() {
            for name in &cli.targets {
                if !config.targets.contains_key(name.as_str()) {
                    return Err(format!("target `{name}` is not defined"));
                }
            }
            cli.targets.iter().map(String::as_str).collect()
        } else if !config.default.is_empty() {
            config.default.iter().map(String::as_str).collect()
        } else {
            config
                .targets
                .iter()
                .filter(|(_, t)| cli.once || t.is_watchable())
                .map(|(n, _)| n.as_str())
                .collect()
        };

        if names.is_empty() {
            return Err(
                "no watchable targets found — add `watch = [...]` to at least one target"
                    .to_string(),
            );
        }

        for &name in &names {
            let target = &config.targets[name];
            if !target.run.is_empty() {
                if let Err(e) = runner::run(&target.run, name) {
                    eprintln!("{e}");
                }
            }
        }

        if cli.once {
            return Ok(());
        }

        let watchable: Vec<&str> = names
            .into_iter()
            .filter(|&n| config.targets[n].is_watchable())
            .collect();

        if watchable.is_empty() {
            return Ok(());
        }

        let ignore = watcher::build_ignore_set(&config.ignore);
        let stop = Arc::new(AtomicBool::new(false));
        let config_changed = Arc::new(AtomicBool::new(false));

        {
            let stop = Arc::clone(&stop);
            let config_changed = Arc::clone(&config_changed);
            let path = config_path.clone();
            std::thread::spawn(move || {
                watcher::watch_config(&path, &stop, &config_changed);
            });
        }

        if watchable.len() == 1 {
            let name = watchable[0];
            watcher::watch(name, &config.targets[name], &ignore, Arc::clone(&stop))?;
        } else {
            std::thread::scope(|s| {
                for &name in &watchable {
                    let target = &config.targets[name];
                    s.spawn({
                        let stop = Arc::clone(&stop);
                        || {
                            if let Err(e) = watcher::watch(name, target, &ignore, stop) {
                                eprintln!("{e}");
                            }
                        }
                    });
                }
            });
        }

        stop.store(true, Ordering::Relaxed);

        if config_changed.load(Ordering::Relaxed) {
            eprintln!("[wat] config changed, reloading...");
            continue;
        }

        return Ok(());
    }
}
