# wat

`wat` is a small cross-platform CLI for rerunning commands when files change.

The config file is TOML named `watfile` or `watfile.toml`.

```toml
default = ["build"]
ignore = [".git/", "target/"]

[build]
watch = ["src/**", "Cargo.toml"]
run = ["cargo build"]

[serve]
watch = ["src/**"]
interrupt = true
run = ["cargo run"]
```

Each target only supports:

* `watch`: paths or globs to watch
* `run`: commands to execute on startup and on changes
* `interrupt`: restart the command immediately when changes arrive

Top-level options:

* `default`: targets used when no target names are passed
* `ignore`: paths to skip

```bash
wat
wat serve
wat build --once
```
