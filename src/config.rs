use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use serde::Deserialize;

pub const DEFAULT_CONFIG_NAMES: [&str; 3] = ["Watfile", "watfile", "Watfile.toml"];

#[derive(Debug, Deserialize)]
pub struct Config {
    #[serde(default = "default_ignore")]
    pub ignore: Vec<String>,
    #[serde(flatten)]
    pub targets: HashMap<String, Target>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Target {
    /// Paths/directories to watch (recursive).
    #[serde(default)]
    pub watch: Vec<String>,
    /// Interrupt (kill) the running process on the next event instead of waiting.
    #[serde(default)]
    pub interrupt: bool,
    /// Commands run on any event (and once on startup).
    #[serde(default)]
    pub run: Vec<String>,
    /// Commands run only on file content changes.
    #[serde(default)]
    pub on_change: Vec<String>,
    /// Commands run only on new files/directories.
    #[serde(default)]
    pub on_create: Vec<String>,
    /// Commands run only on deletions.
    #[serde(default)]
    pub on_delete: Vec<String>,
    /// Commands run only on renames/moves.
    #[serde(default)]
    pub on_rename: Vec<String>,
}

impl Target {
    pub fn watch_paths(&self) -> Vec<PathBuf> {
        self.watch.iter().map(PathBuf::from).collect()
    }

    pub fn is_watchable(&self) -> bool {
        !self.watch.is_empty()
    }

    pub fn has_commands(&self) -> bool {
        !self.run.is_empty()
            || !self.on_change.is_empty()
            || !self.on_create.is_empty()
            || !self.on_delete.is_empty()
            || !self.on_rename.is_empty()
    }
}

fn default_ignore() -> Vec<String> {
    vec![".git/".to_string()]
}

pub fn find_config() -> Result<PathBuf, String> {
    for name in DEFAULT_CONFIG_NAMES {
        let path = PathBuf::from(name);
        if path.is_file() {
            return Ok(path);
        }
    }
    Err("no config file found; tried: Watfile, watfile, Watfile.toml".to_string())
}

pub fn load(path: &Path) -> Result<Config, String> {
    let contents = fs::read_to_string(path)
        .map_err(|e| format!("failed to read `{}`: {e}", path.display()))?;
    let config: Config = toml::from_str(&contents)
        .map_err(|e| format!("{}: {e}", path.display()))?;
    validate(path, &config)?;
    Ok(config)
}

fn validate(path: &Path, config: &Config) -> Result<(), String> {
    if config.targets.is_empty() {
        return Err(format!("{}: no targets defined", path.display()));
    }
    for (name, target) in &config.targets {
        if !target.has_commands() {
            return Err(format!(
                "{}: target `{name}` has no commands (`run`, `on_change`, etc.)",
                path.display()
            ));
        }
    }
    Ok(())
}
