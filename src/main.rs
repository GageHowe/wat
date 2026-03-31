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
        cli::ParseResult::Run(cli) => cli,
        cli::ParseResult::Help => {
            println!("{}", cli::help_text());
            return Ok(());
        }
        cli::ParseResult::Version => {
            println!("{}", cli::version_text());
            return Ok(());
        }
        cli::ParseResult::Update => {
            return update();
        }
    };

    let config_path = cli
        .config_path
        .clone()
        .map(Ok)
        .unwrap_or_else(config::find_config)?;
    let config = config::load(&config_path)?;
    let names = select_targets(&cli, &config)?;

    if names.is_empty() {
        return Err("no watchable targets found".to_string());
    }

    let mut selected = Vec::with_capacity(names.len());
    for name in names {
        selected.push((name, &config.targets[name]));
    }

    run_startup(&selected, cli.once);

    if cli.once {
        return Ok(());
    }

    let watchable: Vec<_> = selected
        .into_iter()
        .filter(|(_, target)| target.is_watchable())
        .collect();
    if watchable.is_empty() {
        return Ok(());
    }

    watcher::watch(&watchable, &config.ignore)
}

fn update() -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        let status = std::process::Command::new("powershell")
            .args(["-Command", "irm https://raw.githubusercontent.com/GageHowe/wat/main/scripts/install.ps1 | iex"])
            .status()
            .map_err(|e| format!("failed to run update: {e}"))?;
        if !status.success() {
            return Err("update failed".to_string());
        }
    }
    #[cfg(target_os = "macos")]
    {
        let status = std::process::Command::new("sh")
            .args(["-c", "curl -fsSL https://raw.githubusercontent.com/GageHowe/wat/main/scripts/install-macos.sh | sh"])
            .status()
            .map_err(|e| format!("failed to run update: {e}"))?;
        if !status.success() {
            return Err("update failed".to_string());
        }
    }
    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    {
        let status = std::process::Command::new("sh")
            .args(["-c", "curl -fsSL https://raw.githubusercontent.com/GageHowe/wat/main/scripts/install.sh | sh"])
            .status()
            .map_err(|e| format!("failed to run update: {e}"))?;
        if !status.success() {
            return Err("update failed".to_string());
        }
    }
    Ok(())
}

fn select_targets<'a>(cli: &'a cli::Cli, config: &'a config::Config) -> Result<Vec<&'a str>, String> {
    if !cli.targets.is_empty() {
        for name in &cli.targets {
            if !config.targets.contains_key(name.as_str()) {
                return Err(format!("target `{name}` is not defined"));
            }
        }
        return Ok(cli.targets.iter().map(String::as_str).collect());
    }

    if !config.default.is_empty() {
        return Ok(config.default.iter().map(String::as_str).collect());
    }

    Ok(config
        .targets
        .iter()
        .filter(|(_, target)| cli.once || target.is_watchable())
        .map(|(name, _)| name.as_str())
        .collect())
}

fn run_startup(selected: &[(&str, &config::Target)], once: bool) {
    for &(name, target) in selected {
        if target.run.is_empty() || (target.interrupt && !once) {
            continue;
        }
        if let Err(e) = runner::run(&target.run, name) {
            eprintln!("[{name}] {e}");
        }
    }
}
