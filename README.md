# wat

`wat` is a tiny cross-platform, language-agnostic CLI for watching file changes and executing commands.

This tool is for my own personal use and may have breaking changes each minor version. If you use it, let me know how it goes!

Running `wat` with no target names uses `default`. If `default` is not set, `wat` exits with an error.

The config file is a TOML file named `watfile` or `watfile.toml`. Example:
```toml
default = ["backend", "frontend"]

[init]
run = ["npm install --prefix frontend", "cargo fetch --manifest-path backend/Cargo.toml"]

[backend]
watch = ["backend/src/**", "backend/Cargo.toml"]
run = ["cd backend && cargo build"]

[frontend]
watch = ["frontend/src/**", "frontend/package.json", "frontend/package-lock.json"]
run = ["npm run build --prefix frontend"]

[serve-frontend]
watch = ["frontend/src/**", "frontend/package.json", "frontend/package-lock.json"]
interrupt = true
run = ["npm run dev --prefix frontend"]

[serve-backend]
watch = ["backend/src/**", "backend/Cargo.toml"]
interrupt = true
run = ["cd backend && cargo run"]
```

Top-level options:
* `default`: targets used when no target names are passed. Just run `wat`
* `ignore`: paths to skip when matching changed files

Each target supports:

* `watch`: paths or globs to watch. If this is not provided, or `--once` is used, the target only runs once.
* `run`: commands to execute on startup and on changes
* `interrupt`: restart the command when changes arrive by killing the direct child process started by `wat` and launching it again. Descendant processes may continue running; commands are responsible for handling termination cleanly themselves.

`run` supports multiple commands. They are chained in the shell with `&&`, so `run = ["cargo fmt --check", "cargo build", "cargo test"]` behaves like `cargo fmt --check && cargo build && cargo test`

## execution model

* If you pass target names, `wat` runs those targets.
* If you pass no target names, `wat` runs `default`.
* If you pass no target names and `default` is not set, `wat` exits with an error.
* Non-`interrupt` targets run to completion on startup and on each matching change.
* `interrupt` targets are started once and restarted when a matching change arrives.
* Each target's `run` list is executed as one shell command joined with `&&`.

If you want two long-running processes, use two targets and run them together instead of putting both in one `run` list.

When chaining commands across sibling directories, prefer commands that do not depend on the previous working directory, such as `npm --prefix ...` or `cargo --manifest-path ...`. Or if you need to use `cd`, handle it explicitly.

`ignore` filters changed paths after the watcher receives events. In other words, ignored directories may still be watched by the OS if they sit under a watched root, but they will not trigger targets once matched by `ignore`.

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

`wat update` automatically grabs these links and updates itself, unless your config defines a target named `update`. `wat -v` shows the current version.
