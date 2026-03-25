# wat

`wat` is a tiny Rust CLI for "run this target whenever these files change".

It borrows two ideas:

- from `make`: a small config file with named targets
- from `watchexec`: rerun commands when files change

## Example

Create a `Watfile`:

```make
watch: src Cargo.toml Watfile
default: test

test:
  cargo test

run:
  cargo run
```

Then run:

```bash
wat
```

Or pick a specific target:

```bash
wat run
```

## Install

Unix-like shells:

```bash
curl -fsSL https://raw.githubusercontent.com/howeg/wat/main/scripts/install.sh | sh
```

PowerShell:

```powershell
irm https://raw.githubusercontent.com/howeg/wat/main/scripts/install.ps1 | iex
```

Both installers support:

- `WAT_INSTALL_REPO` to point at a fork or alternate GitHub repo
- `WAT_INSTALL_VERSION` to install a specific tag instead of the latest release
- `WAT_INSTALL_BIN` to choose the destination directory
- `WAT_INSTALL_SET_PATH=0` to skip shell profile updates on Unix

The PowerShell installer also supports `-NoPathUpdate` if you only want a session-local PATH change.

What the scripts do:

- detect the current OS and CPU architecture
- download the matching GitHub Release artifact for `wat`
- extract the binary into your chosen install directory
- add that directory to PATH when possible

## Release Packaging

Build the archives expected by the install scripts:

```bash
scripts/package-release.sh x86_64-unknown-linux-musl
scripts/package-release.sh aarch64-apple-darwin
```

```powershell
.\scripts\package-release.ps1 -Target x86_64-pc-windows-msvc
```

The expected release asset names are:

- `wat-x86_64-unknown-linux-musl.tar.gz`
- `wat-aarch64-apple-darwin.tar.gz`
- `wat-x86_64-pc-windows-msvc.zip`
- `wat-aarch64-pc-windows-msvc.zip`

## Config format

- `watch:` lists files or directories separated by spaces
- `default:` picks the target used by `wat` with no arguments
- `target:` starts a target block
- indented lines under a target are shell commands

Notes:

- if `watch:` is omitted, `wat` watches the current directory
- the config file is always watched too
- `target` and `.git` directories are skipped while scanning

## CLI

```bash
wat [target] [--once]
wat --file ./Watfile [target] [--once]
```
