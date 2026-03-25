# Contributing

## Requirements

- Rust toolchain installed
- Git
- PowerShell on Windows or a POSIX shell on macOS/Linux

## Local development

Build the project:

```bash
cargo build
```

Run the sample target once:

```bash
cargo run -- --once
```

Show CLI help:

```bash
cargo run -- --help
```

Run tests:

```bash
cargo test
```

## Project files

- `Watfile` is a sample config for local development
- `scripts/install.sh` installs the latest Linux release
- `scripts/install-macos.sh` installs the latest macOS release
- `scripts/install.ps1` installs the latest Windows release
- `scripts/package-release.sh` builds `.tar.gz` archives for Unix targets
- `scripts/package-release.ps1` builds `.zip` archives for Windows targets
- `.github/workflows/release.yml` publishes release assets from Git tags

## Building release archives locally

Linux:

```bash
scripts/package-release.sh x86_64-unknown-linux-musl
```

macOS:

```bash
scripts/package-release.sh aarch64-apple-darwin
scripts/package-release.sh x86_64-apple-darwin
```

Windows:

```powershell
.\scripts\package-release.ps1 -Target x86_64-pc-windows-msvc
```

The generated archives are written to `dist/`.

## Releasing

GitHub Releases are automated by `.github/workflows/release.yml`.

To publish a release:

```bash
git tag v0.1.0
git push origin v0.1.0
```

That workflow builds and uploads:

- `wat-x86_64-unknown-linux-musl.tar.gz`
- `wat-aarch64-apple-darwin.tar.gz`
- `wat-x86_64-apple-darwin.tar.gz`
- `wat-x86_64-pc-windows-msvc.zip`

## Notes

- The public install scripts are intentionally zero-config and choose the default install location for each operating system.
- Windows ARM packaging is not automated yet.
