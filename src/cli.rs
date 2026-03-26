use std::path::PathBuf;

#[derive(Debug)]
pub struct Cli {
    pub config_path: Option<PathBuf>,
    pub targets: Vec<String>,
    pub once: bool,
}

pub fn parse() -> Result<Option<Cli>, String> {
    use lexopt::prelude::*;

    let mut cli = Cli {
        config_path: None,
        targets: Vec::new(),
        once: false,
    };
    let mut parser = lexopt::Parser::from_env();

    while let Some(arg) = parser.next().map_err(|e| e.to_string())? {
        match arg {
            Short('h') | Long("help") => return Ok(None),
            Short('f') | Long("file") => {
                cli.config_path = Some(PathBuf::from(parser.value().map_err(|e| e.to_string())?));
            }
            Long("once") => cli.once = true,
            Short(c) => return Err(format!("unknown flag `-{c}`")),
            Long(s) => return Err(format!("unknown flag `--{s}`")),
            Value(v) => cli.targets.push(v.to_string_lossy().into_owned()),
        }
    }

    Ok(Some(cli))
}

pub fn help_text() -> &'static str {
    "wat - a tiny config-driven file watcher

USAGE:
  wat [target...] [--once]
  wat --file ./watfile.toml [target...] [--once]

FLAGS:
  -f, --file <path>   Use a specific config file
      --once          Run target(s) once without watching
  -h, --help          Show this help text

EXAMPLES:
  wat
  wat build
  wat backend tests
  wat build --once"
}
