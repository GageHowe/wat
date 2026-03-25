#!/usr/bin/env sh
set -eu

REPO="howeg/wat"
BIN_DIR="$HOME/.local/bin"
PROFILE="$HOME/.profile"

need_cmd() {
  if ! command -v "$1" >/dev/null 2>&1; then
    echo "error: missing required command: $1" >&2
    exit 1
  fi
}

download() {
  url="$1"
  out="$2"

  if command -v curl >/dev/null 2>&1; then
    curl --fail --location --silent --show-error "$url" --output "$out"
    return
  fi

  if command -v wget >/dev/null 2>&1; then
    wget -qO "$out" "$url"
    return
  fi

  echo "error: install requires curl or wget" >&2
  exit 1
}

ensure_path_line() {
  export_line="export PATH=\"$BIN_DIR:\$PATH\""

  mkdir -p "$(dirname "$PROFILE")"
  touch "$PROFILE"

  if grep -F "$export_line" "$PROFILE" >/dev/null 2>&1; then
    return
  fi

  {
    printf "\n"
    printf "# Added by wat installer\n"
    printf "%s\n" "$export_line"
  } >> "$PROFILE"
}

need_cmd tar
need_cmd mktemp
need_cmd install

if [ "$(uname -s)" != "Linux" ]; then
  echo "error: scripts/install.sh is for Linux. Use scripts/install-macos.sh on macOS." >&2
  exit 1
fi

arch="$(uname -m)"
case "$arch" in
  x86_64|amd64) target="x86_64-unknown-linux-musl" ;;
  *)
    echo "error: unsupported Linux architecture: $arch" >&2
    exit 1
    ;;
esac

archive="wat-${target}.tar.gz"
url="https://github.com/${REPO}/releases/latest/download/${archive}"
tmpdir="$(mktemp -d)"
trap 'rm -rf "$tmpdir"' EXIT INT TERM

mkdir -p "$BIN_DIR"
echo "installing wat from ${url}"
download "$url" "$tmpdir/$archive"
tar -xzf "$tmpdir/$archive" -C "$tmpdir"
install "$tmpdir/wat" "$BIN_DIR/wat"

echo "wat installed to $BIN_DIR/wat"
case ":$PATH:" in
  *":$BIN_DIR:"*) ;;
  *)
    ensure_path_line
    echo "added $BIN_DIR to PATH in $PROFILE"
    echo "restart your shell or run: export PATH=\"$BIN_DIR:\$PATH\""
    ;;
esac
