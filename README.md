# wat

`wat` is a tiny, cross-platform, language-agnostic, hot-reloading CLI for running commands whenever files change, like a mix between `make` and `watchexec`.

This is intended for my personal use. If you use it, let me know what you think.

The config file is TOML, `watfile` or `watfile.toml`. Each target is a named section:
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

### top-level options
* `default`:  Targets to run when none are specified on the command line. If unset, all watchable targets run.
* `ignore`: Override paths to skip. `/` specifies directories, does not need glob syntax.

### target-specific options
* `watch`: Paths or globs to watch. Targets with no `watch` are run-only.
* `run` | `[]` | Commands run on any event and once at startup.
* `interrupt`: Kill the running process on the next event instead of waiting.
* `on_change`: Commands run only when file content changes.
* `on_create`: Commands run only when a file or directory is created.
* `on_delete`: Commands run only when a file or directory is deleted.
* `on_rename`: Commands run only when a file or directory is renamed or moved.

`run` fires first on any event; event-specific handlers fire after.

### example
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

## install
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
