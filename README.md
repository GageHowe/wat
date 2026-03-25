![wat](https://github.com/user-attachments/assets/23d8c9d2-03dd-43a0-b9f7-09c5e824bb5c)


`wat` is a tiny, cross-platform, language-agnostic CLI for running commands whenever files change.

Uses the `notify` crate to efficiently watch for changes.

## Example

```toml
[build]
watch = ["src", "Cargo.toml"]
run = ["cargo build"]

[test]
watch = ["src", "Cargo.toml", "Watfile"]
run = ["cargo test"]

[run]
watch = ["src"]
interrupt = true
run = ["cargo run"]
```

Watch all targets simultaneously:

```bash
wat
```

Watch a specific target:

```bash
wat build
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

## Watfile format

The config file is TOML. Each target is a named section:

```toml
[name]
watch = ["path1", "path2"]   # directories or files, watched recursively
run   = ["command", "..."]   # runs on any event (and once on startup)
```

Targets with no `watch` paths are run-only (use with `--once`).

### Top-level options

| Key | Default | Description |
|-----|---------|-------------|
| `ignore` | `[".git/"]` | Paths to skip. Supports glob patterns. |

```toml
ignore = [".git/", "target/", "*.log"]
```

Patterns ending with `/` match any directory with that name (and its contents).
Plain patterns match any file or directory with that name. Standard glob syntax
(`*.log`, `**/__pycache__`) is also supported.

### Target options

| Key | Default | Description |
|-----|---------|-------------|
| `watch` | `[]` | Paths to watch recursively. |
| `run` | `[]` | Commands run on any event and once at startup. |
| `interrupt` | `false` | Kill the running process on the next event instead of waiting for it to finish. Useful for long-running servers. |

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
watch = ["dist"]
on_change = ["rsync -r dist/ server:/var/www"]
on_create = ["rsync -r dist/ server:/var/www"]
```

### Multiple watch groups

Run independent groups in parallel — useful for monorepos or multi-package workspaces:

```toml
[backend]
watch = ["backend/src", "backend/Cargo.toml"]
interrupt = true
run = ["cargo build -p backend"]

[frontend]
watch = ["frontend/src", "frontend/package.json"]
interrupt = true
run = ["npm run build"]

[tests]
watch = ["backend/src", "frontend/src"]
run = ["cargo test --workspace"]

[clean]
run = ["cargo clean", "rm -rf frontend/dist"]
```

```bash
wat                  # starts all watchable targets simultaneously
wat backend          # watches only the backend target
wat backend frontend # watches both targets simultaneously
wat clean --once     # runs clean once and exits
```

## CLI

```
wat [target...] [--once]
wat --file ./path/to/Watfile [target...] [--once]

  -f, --file <path>   Use a specific config file (default: Watfile)
      --once          Run target(s) once without watching
  -h, --help          Show this help text
```
