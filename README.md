![wat](https://github.com/user-attachments/assets/23d8c9d2-03dd-43a0-b9f7-09c5e824bb5c)


`wat` is a tiny, cross-platform, language-agnostic CLI for running commands whenever files change, inspired by `make` and `watchexec`.

## Config format

The config file is TOML. Each target is a named section:

```toml
[name]
watch = ["path1", "path2", "file1"]  # directories or files (surface-level by default)
run   = ["command", "..."]           # runs on any event and once on startup
```

Plain paths watch only the top level of a directory. Use glob syntax to go deeper:

```toml
watch = ["src/*"]      # files directly in src/
watch = ["src/**"]     # all files in src/ recursively
watch = ["src/**/*.rs"] # only .rs files, recursively
```

Targets with no `watch` are run-only — useful as command shortcuts alongside `--once`.

Example:

```toml
[client]
watch = ["client/src/**", "client/Cargo.toml", "Cargo.toml"]
run = ["cargo build -p client"]

[server]
watch = ["server/src/**", "server/Cargo.toml", "Cargo.toml"]
run = ["cargo build -p server"]

[test]
run = ["cargo test"]
```

```bash
wat                  # watches client + server simultaneously
wat client           # watches only client
wat test --once      # runs cargo test once and exits
```

### Ignore

Skip paths at the top level of the config:

```toml
ignore = [".git/", "target/", "*.log"]
```

Patterns ending with `/` match any directory with that name (and its contents).
Plain patterns match any file or directory with that name anywhere in the tree.
Standard glob syntax (`*.log`, `**/__pycache__`) is also accepted.

Default: `[".git/"]`

### Target options

| Key | Default | Description |
|-----|---------|-------------|
| `watch` | `[]` | Paths or globs to watch. |
| `run` | `[]` | Commands run on any event and once at startup. |
| `interrupt` | `false` | Kill the running process on the next event instead of waiting. Useful for long-running servers. |

### Event handlers

For finer control, commands can be scoped to a specific event type. Plain `run`
commands fire first on any event; matching handlers fire after.

| Key | Fires on |
|-----|----------|
| `on_change` | File content modified |
| `on_create` | File or directory created |
| `on_delete` | File or directory deleted |
| `on_rename` | File or directory renamed / moved |

```toml
[sync]
watch = ["dist/**"]
on_change = ["rsync -r dist/ server:/var/www"]
on_create = ["rsync -r dist/ server:/var/www"]
```

## CLI

```
wat [target...] [--once]
wat --file ./path/to/Watfile [target...] [--once]

  -f, --file <path>   Use a specific config file (default: Watfile / watfile / Watfile.toml)
      --once          Run target(s) once without watching
  -h, --help          Show this help text
```

## Install

Linux
```bash
curl -fsSL https://raw.githubusercontent.com/GageHowe/wat/main/scripts/install.sh | sh
```

MacOS
```bash
curl -fsSL https://raw.githubusercontent.com/GageHowe/wat/main/scripts/install-macos.sh | sh
```

Windows
```powershell
irm https://raw.githubusercontent.com/GageHowe/wat/main/scripts/install.ps1 | iex
```
