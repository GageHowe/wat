use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use serde::Deserialize;

pub const DEFAULT_CONFIG_NAMES: [&str; 2] = ["watfile", "watfile.toml"];

#[derive(Debug, Deserialize)]
pub struct Config {
    #[serde(default = "default_ignore")]
    pub ignore: Vec<String>,
    #[serde(default)]
    pub default: Vec<String>,
    #[serde(flatten)]
    pub targets: HashMap<String, Target>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Target {
    #[serde(default)]
    pub watch: Vec<String>,
    #[serde(default)]
    pub interrupt: bool,
    #[serde(default)]
    pub run: Vec<String>,
}
impl Target {
    pub fn is_watchable(&self) -> bool {
        !self.watch.is_empty()
    }
}

fn default_ignore() -> Vec<String> {
    vec![".git/".to_string()] // thought this was a sane default, but might make it empty
}

pub fn find_config() -> Result<PathBuf, String> {
    for name in DEFAULT_CONFIG_NAMES {
        let path = PathBuf::from(name);
        if path.is_file() {
            return Ok(path);
        }
    }
    Err("no watfile found".to_string())
}

pub fn load(path: &Path) -> Result<Config, String> {
    let contents = fs::read_to_string(path)
        .map_err(|e| format!("failed to read `{}`: {e}", path.display()))?;
    let config: Config =
        toml::from_str(&contents).map_err(|e| format!("{}: {e}", path.display()))?;
    validate(path, &config)?;
    Ok(config)
}

fn validate(path: &Path, config: &Config) -> Result<(), String> {
    if config.targets.is_empty() {
        return Err(format!("{}: no targets defined", path.display()));
    }
    for (name, target) in &config.targets {
        if target.run.is_empty() {
            return Err(format!(
                "{}: target `{name}` has no run commands",
                path.display()
            ));
        }
    }
    for name in &config.default {
        if !config.targets.contains_key(name.as_str()) {
            return Err(format!(
                "{}: `default` references undefined target `{name}`",
                path.display()
            ));
        }
    }
    Ok(())
}
