![wat](https://github.com/user-attachments/assets/23d8c9d2-03dd-43a0-b9f7-09c5e824bb5c)


`wat` is a tiny, cross-platform, language-agnostic CLI for running commands whenever files change.

It borrows two ideas:
- `make`'s small config file with named targets
- `watchexec`'s ability to rerun commands when files change

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

MacOS/Linux
```bash
curl -fsSL https://raw.githubusercontent.com/howeg/wat/main/scripts/install.sh | sh
```

Windows
```powershell
irm https://raw.githubusercontent.com/howeg/wat/main/scripts/install.ps1 | iex
```

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
