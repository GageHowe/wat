use std::path::PathBuf;

#[derive(Debug)]
pub struct Cli {
    pub config_path: Option<PathBuf>,
    /// Explicit target names from the command line (empty = watch all)
    pub targets: Vec<String>,
    pub once: bool,
}

pub enum Outcome {
    Run(Cli),
    Help,
}

pub fn parse() -> Result<Outcome, String> {
    use lexopt::prelude::*;

    let mut cli = Cli {
        config_path: None,
        targets: Vec::new(),
        once: false,
    };
    let mut parser = lexopt::Parser::from_env();

    while let Some(arg) = parser.next().map_err(|e| e.to_string())? {
        match arg {
            Short('h') | Long("help") => return Ok(Outcome::Help),
            Short('f') | Long("file") => {
                cli.config_path = Some(PathBuf::from(parser.value().map_err(|e| e.to_string())?));
            }
            Long("once") => cli.once = true,
            Short(c) => return Err(format!("unknown flag `-{c}`")),
            Long(s) => return Err(format!("unknown flag `--{s}`")),
            Value(v) => cli.targets.push(v.to_string_lossy().into_owned()),
        }
    }

    Ok(Outcome::Run(cli))
}

pub fn help_text() -> &'static str {
    "wat - a tiny config-driven file watcher

USAGE:
  wat [target...] [--once]
  wat --file ./Watfile [target...] [--once]

FLAGS:
  -f, --file <path>   Use a specific config file
      --once          Run target(s) once without watching
  -h, --help          Show this help text

EXAMPLES:
  wat                 Watch all groups simultaneously
  wat build           Watch only the `build` group
  wat backend tests   Watch both groups simultaneously
  wat build --once    Run `build` once and exit"
}
