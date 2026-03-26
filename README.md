# wat

`wat` is a tiny cross-platform, language-agnostic CLI for watching file changes and executing commands.

This tool is for my own personal use and may have breaking changes each minor version. If you use it, let me know how it goes!

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
* `ignore`: paths to skip

Each target supports:

* `watch`: paths or globs to watch. If this is not provided, or `--once` is used, the target only runs once.
* `run`: commands to execute on startup and on changes
* `interrupt`: kill the child process immediately when changes arrive. You'd want this for long-running processes, and I'm considering making this the default behavior.

`run` supports multiple commands. They are chained in the shell with `&&`, so `run = ["cargo fmt --check", "cargo build", "cargo test"]` behaves like `cargo fmt --check && cargo build && cargo test`

If you want two long-running processes, use two targets and run them together instead of putting both in one `run` list.

When chaining commands across sibling directories, prefer commands that do not depend on the previous working directory, such as `npm --prefix ...` or `cargo --manifest-path ...`. Or if you need to use `cd`, handle it explicitly. Though this isn't very clean and I'm thinking about how to handle cases like this better.
