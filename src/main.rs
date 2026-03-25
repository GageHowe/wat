mod cli;
mod config;
mod runner;
mod watcher;

use std::process::ExitCode;

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
        .map(Ok)
        .unwrap_or_else(config::find_config)?;

    let config = config::load(&config_path)?;

    // Resolve which targets to activate
    let names: Vec<&str> = if !cli.targets.is_empty() {
        for name in &cli.targets {
            if !config.targets.contains_key(name.as_str()) {
                return Err(format!("target `{name}` is not defined"));
            }
        }
        cli.targets.iter().map(String::as_str).collect()
    } else {
        // Default: every target that has watch paths (or every target for --once)
        config
            .targets
            .iter()
            .filter(|(_, t)| cli.once || t.is_watchable())
            .map(|(n, _)| n.as_str())
            .collect()
    };

    if names.is_empty() {
        return Err(
            "no watchable targets found — add `watch = [...]` to at least one target".to_string(),
        );
    }

    // Run startup commands once before watching
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

    // Filter to watchable targets only
    let watchable: Vec<&str> = names
        .into_iter()
        .filter(|&n| config.targets[n].is_watchable())
        .collect();

    if watchable.is_empty() {
        return Ok(());
    }

    let ignore = watcher::build_ignore_set(&config.ignore);

    if watchable.len() == 1 {
        let name = watchable[0];
        return watcher::watch(name, &config.targets[name], &ignore);
    }

    std::thread::scope(|s| {
        for &name in &watchable {
            let target = &config.targets[name];
            s.spawn(|| {
                if let Err(e) = watcher::watch(name, target, &ignore) {
                    eprintln!("{e}");
                }
            });
        }
    });

    Ok(())
}
