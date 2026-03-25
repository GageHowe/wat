![wat](https://github.com/user-attachments/assets/23d8c9d2-03dd-43a0-b9f7-09c5e824bb5c)


`wat` is a tiny, cross-platform, language-agnostic, hot-reloading CLI for running commands whenever files change, inspired by `make` and `watchexec`.

The config file is TOML. Each target is a named section:
```toml
[name]
watch = ["path1", "path2"]  # directories or files
run   = ["command", "..."]  # runs on any event and once at startup
```
Glob syntax is supported:
```toml
watch = ["src/*"]       # files directly in src/
watch = ["src/**"]      # all files in src/ recursively
watch = ["src/**/*.rs"] # only .rs files, recursively
```

### Target options
* `watch`: Paths or globs to watch. Targets with no `watch` are run-only.
`run` | `[]` | Commands run on any event and once at startup.
`interrupt`: Kill the running process on the next event instead of waiting.
`on_change`: Commands run only when file content changes.
`on_create`: Commands run only when a file or directory is created.
`on_delete`: Commands run only when a file or directory is deleted.
`on_rename`: Commands run only when a file or directory is renamed or moved.

`run` fires first on any event; event-specific handlers fire after.

### Top-level options
* `default`:  Targets to run when none are specified on the command line. If unset, all watchable targets run.
* `ignore`: Paths to skip. Patterns ending in `/` match directories.

### Example
```toml
default = ["client", "server"]

ignore = [".git/", "target/"]

[client]
watch = ["client/src/**", "client/Cargo.toml"]
run = ["cargo build -p client"]

[server]
watch = ["server/src/**", "server/Cargo.toml"]
run = ["cargo build -p server"]

[test]
watch = ["src/**"]
run = ["cargo test"]
```

```bash
wat             # watches client + server (the default set)
wat test        # watches only test
wat test --once # runs test once and exits
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
