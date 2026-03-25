#!/usr/bin/env sh
set -eu

TARGET="${1:-}"

if [ -z "$TARGET" ]; then
  echo "usage: scripts/package-release.sh <target-triple>" >&2
  exit 1
fi

if ! command -v cargo >/dev/null 2>&1; then
  echo "error: cargo is required" >&2
  exit 1
fi

VERSION="$(sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -n 1)"
OUT_DIR="dist"
STAGE_DIR="$OUT_DIR/wat-$VERSION-$TARGET"
ARCHIVE="$OUT_DIR/wat-$TARGET.tar.gz"

rm -rf "$STAGE_DIR"
mkdir -p "$STAGE_DIR"
mkdir -p "$OUT_DIR"

cargo build --release --target "$TARGET"
cp "target/$TARGET/release/wat" "$STAGE_DIR/wat"
cp README.md "$STAGE_DIR/README.md"

tar -czf "$ARCHIVE" -C "$STAGE_DIR" .
echo "wrote $ARCHIVE"
