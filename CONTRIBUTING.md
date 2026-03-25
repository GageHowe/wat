# Contributing

## Requirements
- Rust toolchain installed
- Git
- PowerShell on Windows or a POSIX shell on macOS/Linux

## Project files
- `Watfile` is a sample config for local development
- `scripts/install.sh` installs the latest Linux release
- `scripts/install-macos.sh` installs the latest macOS release
- `scripts/install.ps1` installs the latest Windows release
- `scripts/package-release.sh` builds `.tar.gz` archives for Unix targets
- `scripts/package-release.ps1` builds `.zip` archives for Windows targets
- `.github/workflows/release.yml` publishes release assets from Git tags

## Releasing
GitHub Releases are automated by `.github/workflows/release.yml`. To publish a release:

```bash
git tag version
git push origin version
```

Builds:
- `wat-x86_64-unknown-linux-musl.tar.gz`
- `wat-aarch64-apple-darwin.tar.gz`
- `wat-x86_64-apple-darwin.tar.gz`
- `wat-x86_64-pc-windows-msvc.zip`
