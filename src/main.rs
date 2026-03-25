use std::collections::{BTreeMap, HashMap};
use std::env;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};
use std::thread;
use std::time::{Duration, SystemTime};

const DEFAULT_CONFIG_NAMES: [&str; 3] = ["Watfile", "watfile", "Watfile.toml"];
const POLL_INTERVAL: Duration = Duration::from_millis(500);

#[derive(Debug)]
struct AppConfig {
    path: PathBuf,
    default_target: Option<String>,
    watch_paths: Vec<PathBuf>,
    targets: HashMap<String, Vec<String>>,
    first_target: Option<String>,
}

#[derive(Debug)]
struct Cli {
    config_path: Option<PathBuf>,
    target: Option<String>,
    once: bool,
}

#[derive(Debug)]
enum CliError {
    Message(String),
}

#[derive(Debug)]
enum ConfigError {
    Io(io::Error),
    Message(String),
}

impl From<io::Error> for ConfigError {
    fn from(error: io::Error) -> Self {
        Self::Io(error)
    }
}

fn main() -> ExitCode {
    match run() {
        Ok(code) => code,
        Err(message) => {
            eprintln!("error: {message}");
            ExitCode::from(1)
        }
    }
}

fn run() -> Result<ExitCode, String> {
    let cli = match parse_cli() {
        Ok(ParseOutcome::Run(cli)) => cli,
        Ok(ParseOutcome::Help) => {
            println!("{}", help_text());
            return Ok(ExitCode::SUCCESS);
        }
        Err(error) => return Err(cli_error(error)),
    };
    let config_path = cli
        .config_path
        .map(Ok)
        .unwrap_or_else(find_config)
        .map_err(config_error)?;
    let config = load_config(&config_path).map_err(config_error)?;
    let target = resolve_target(&config, cli.target.as_deref())?;

    run_target(&config, &target)?;

    if cli.once {
        return Ok(ExitCode::SUCCESS);
    }

    let watch_paths = resolve_watch_paths(&config);
    let mut snapshot = build_snapshot(&watch_paths);

    println!("watching {} path(s) for target `{target}`", watch_paths.len());

    loop {
        thread::sleep(POLL_INTERVAL);
        let next = build_snapshot(&watch_paths);
        if let Some(reason) = diff_snapshots(&snapshot, &next) {
            println!();
            println!("change detected: {reason}");
            if let Err(error) = run_target(&config, &target) {
                eprintln!("target failed: {error}");
            }
            snapshot = next;
        }
    }
}

enum ParseOutcome {
    Run(Cli),
    Help,
}

fn parse_cli() -> Result<ParseOutcome, CliError> {
    let mut args = env::args_os().skip(1);
    let mut cli = Cli {
        config_path: None,
        target: None,
        once: false,
    };

    while let Some(arg) = args.next() {
        match arg.to_string_lossy().as_ref() {
            "-h" | "--help" => return Ok(ParseOutcome::Help),
            "-f" | "--file" => {
                let value = args.next().ok_or_else(|| {
                    CliError::Message("expected a path after `-f` / `--file`".to_string())
                })?;
                cli.config_path = Some(PathBuf::from(value));
            }
            "--once" => cli.once = true,
            flag if flag.starts_with('-') => {
                return Err(CliError::Message(format!("unknown flag `{flag}`")));
            }
            value => {
                if cli.target.is_some() {
                    return Err(CliError::Message(format!(
                        "unexpected extra argument `{value}`"
                    )));
                }
                cli.target = Some(value.to_string());
            }
        }
    }

    Ok(ParseOutcome::Run(cli))
}

fn help_text() -> String {
    [
        "wat - a tiny config-driven file watcher",
        "",
        "USAGE:",
        "  wat [target] [--once]",
        "  wat --file ./Watfile [target] [--once]",
        "",
        "FLAGS:",
        "  -f, --file <path>   Use a specific config file",
        "      --once          Run the target once without watching",
        "  -h, --help          Show this help text",
    ]
    .join("\n")
}

fn find_config() -> Result<PathBuf, ConfigError> {
    for candidate in DEFAULT_CONFIG_NAMES {
        let path = PathBuf::from(candidate);
        if path.is_file() {
            return Ok(path);
        }
    }

    Err(ConfigError::Message(
        "could not find a config file. Tried: Watfile, watfile, Watfile.toml".to_string(),
    ))
}

fn load_config(path: &Path) -> Result<AppConfig, ConfigError> {
    let contents = fs::read_to_string(path)?;
    let mut config = AppConfig {
        path: path.to_path_buf(),
        default_target: None,
        watch_paths: Vec::new(),
        targets: HashMap::new(),
        first_target: None,
    };

    let mut current_target: Option<String> = None;

    for (index, raw_line) in contents.lines().enumerate() {
        let line_number = index + 1;
        let trimmed = raw_line.trim();

        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }

        let is_indented = raw_line.starts_with(' ') || raw_line.starts_with('\t');
        if is_indented {
            let target = current_target.clone().ok_or_else(|| {
                ConfigError::Message(format!(
                    "{}:{}: found an indented command before any target",
                    path.display(),
                    line_number
                ))
            })?;

            config
                .targets
                .get_mut(&target)
                .expect("current target should exist")
                .push(trimmed.to_string());
            continue;
        }

        current_target = None;

        let Some((name, rest)) = trimmed.split_once(':') else {
            return Err(ConfigError::Message(format!(
                "{}:{}: expected `name: value` or `target:`",
                path.display(),
                line_number
            )));
        };

        let key = name.trim();
        let value = rest.trim();

        match key {
            "watch" => {
                config.watch_paths.extend(
                    value
                        .split_whitespace()
                        .filter(|item| !item.is_empty())
                        .map(PathBuf::from),
                );
            }
            "default" => {
                if value.is_empty() {
                    return Err(ConfigError::Message(format!(
                        "{}:{}: `default:` must name a target",
                        path.display(),
                        line_number
                    )));
                }
                config.default_target = Some(value.to_string());
            }
            _ => {
                if !value.is_empty() {
                    return Err(ConfigError::Message(format!(
                        "{}:{}: targets cannot have inline values; put commands on indented lines",
                        path.display(),
                        line_number
                    )));
                }

                if config.targets.contains_key(key) {
                    return Err(ConfigError::Message(format!(
                        "{}:{}: duplicate target `{key}`",
                        path.display(),
                        line_number
                    )));
                }

                config.targets.insert(key.to_string(), Vec::new());
                if config.first_target.is_none() {
                    config.first_target = Some(key.to_string());
                }
                current_target = Some(key.to_string());
            }
        }
    }

    if config.targets.is_empty() {
        return Err(ConfigError::Message(format!(
            "{}: no targets found",
            path.display()
        )));
    }

    if let Some(default_target) = &config.default_target
        && !config.targets.contains_key(default_target)
    {
        return Err(ConfigError::Message(format!(
            "{}: default target `{default_target}` is not defined",
            path.display()
        )));
    }

    for (target, commands) in &config.targets {
        if commands.is_empty() {
            return Err(ConfigError::Message(format!(
                "{}: target `{target}` has no commands",
                path.display()
            )));
        }
    }

    Ok(config)
}

fn resolve_target(config: &AppConfig, requested: Option<&str>) -> Result<String, String> {
    if let Some(target) = requested {
        if config.targets.contains_key(target) {
            return Ok(target.to_string());
        }
        return Err(format!("target `{target}` is not defined"));
    }

    if let Some(target) = &config.default_target {
        return Ok(target.clone());
    }

    config
        .first_target
        .clone()
        .ok_or_else(|| "config has no targets".to_string())
}

fn resolve_watch_paths(config: &AppConfig) -> Vec<PathBuf> {
    let mut paths = if config.watch_paths.is_empty() {
        vec![PathBuf::from(".")]
    } else {
        config.watch_paths.clone()
    };

    if !paths.iter().any(|path| path == &config.path) {
        paths.push(config.path.clone());
    }

    paths
}

fn run_target(config: &AppConfig, target: &str) -> Result<(), String> {
    let commands = config
        .targets
        .get(target)
        .ok_or_else(|| format!("target `{target}` is not defined"))?;

    println!();
    println!("==> {target}");

    for command in commands {
        println!("$ {command}");
        let status = shell_command(command)
            .status()
            .map_err(|error| format!("failed to spawn `{command}`: {error}"))?;
        if !status.success() {
            return Err(format!("command exited with status {status}: `{command}`"));
        }
    }

    Ok(())
}

fn shell_command(command: &str) -> Command {
    if cfg!(windows) {
        let mut cmd = Command::new("cmd");
        cmd.arg("/C").arg(command);
        cmd
    } else {
        let mut cmd = Command::new("sh");
        cmd.arg("-c").arg(command);
        cmd
    }
}

fn build_snapshot(paths: &[PathBuf]) -> BTreeMap<PathBuf, SystemTime> {
    let mut files = BTreeMap::new();

    for path in paths {
        collect_path(path, &mut files);
    }

    files
}

fn collect_path(path: &Path, files: &mut BTreeMap<PathBuf, SystemTime>) {
    let metadata = match fs::metadata(path) {
        Ok(metadata) => metadata,
        Err(_) => return,
    };

    if metadata.is_file() {
        if let Ok(modified) = metadata.modified() {
            files.insert(path.to_path_buf(), modified);
        }
        return;
    }

    if !metadata.is_dir() {
        return;
    }

    let entries = match fs::read_dir(path) {
        Ok(entries) => entries,
        Err(_) => return,
    };

    for entry in entries.flatten() {
        let entry_path = entry.path();
        if should_skip(&entry_path) {
            continue;
        }
        collect_path(&entry_path, files);
    }
}

fn should_skip(path: &Path) -> bool {
    path.file_name()
        .map(|name| {
            let name = name.to_string_lossy();
            name == ".git" || name == "target"
        })
        .unwrap_or(false)
}

fn diff_snapshots(
    previous: &BTreeMap<PathBuf, SystemTime>,
    next: &BTreeMap<PathBuf, SystemTime>,
) -> Option<String> {
    for (path, modified) in next {
        match previous.get(path) {
            None => return Some(format!("new file {}", path.display())),
            Some(old) if old != modified => return Some(format!("updated {}", path.display())),
            Some(_) => {}
        }
    }

    for path in previous.keys() {
        if !next.contains_key(path) {
            return Some(format!("removed {}", path.display()));
        }
    }

    None
}

fn cli_error(error: CliError) -> String {
    match error {
        CliError::Message(message) => message,
    }
}

fn config_error(error: ConfigError) -> String {
    match error {
        ConfigError::Io(inner) => inner.to_string(),
        ConfigError::Message(message) => message,
    }
}
