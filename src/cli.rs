use std::path::PathBuf;

#[derive(Debug)]
pub struct Cli {
    pub config_path: Option<PathBuf>,
    pub targets: Vec<String>,
    pub once: bool,
}

pub enum ParseResult {
    Run(Cli),
    Help,
    Version,
}

pub fn parse() -> Result<ParseResult, String> {
    use lexopt::prelude::*;

    let mut cli = Cli {
        config_path: None,
        targets: Vec::new(),
        once: false,
    };
    let mut parser = lexopt::Parser::from_env();

    // lexopt is insanely nice
    while let Some(arg) = parser.next().map_err(|e| e.to_string())? {
        match arg {
            Short('h') | Long("help") => return Ok(ParseResult::Help),
            Short('v') | Long("version") => return Ok(ParseResult::Version),
            Short('f') | Long("file") => {
                cli.config_path = Some(PathBuf::from(parser.value().map_err(|e| e.to_string())?));
            }
            Long("once") => cli.once = true,
            Short(c) => return Err(format!("unknown flag `-{c}`")),
            Long(s) => return Err(format!("unknown flag `--{s}`")),
            Value(v) => {
                cli.targets.push(v.to_string_lossy().into_owned());
            }
        }
    }

    Ok(ParseResult::Run(cli))
}

#[inline]
pub fn help_text() -> &'static str {
    "wat - a tiny config-driven file watcher

USAGE:
  wat [target...] [--once]
  wat --file ./watfile.toml [target...] [--once]

FLAGS:
  -f, --file <path>   Use a specific config file
  -v, --version       Show the version
      --once          Run target(s) once without watching
  -h, --help          Show this help text

SUBCOMMANDS:
  update              Update wat to the latest version when no target named `update` exists

EXAMPLES:
  wat
  wat build
  wat backend tests
  wat build --once"
}

pub fn version_text() -> &'static str {
    concat!("wat ", env!("CARGO_PKG_VERSION"))
}
