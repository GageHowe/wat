#!/usr/bin/env sh
set -eu

REPO="GageHowe/wat"
BIN_DIR="$HOME/.local/bin"

need_cmd() {
  command -v "$1" >/dev/null 2>&1 || {
    echo "error: missing required command: $1" >&2
    exit 1
  }
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

detect_profile() {
  # explicit shell env
  if [ -n "${ZSH_VERSION-}" ]; then
    echo "$HOME/.zshrc"
    return
  fi

  if [ -n "${BASH_VERSION-}" ]; then
    echo "$HOME/.bashrc"
    return
  fi

  # fallback to SHELL
  case "${SHELL-}" in
    */zsh) echo "$HOME/.zshrc" ;;
    */bash) echo "$HOME/.bashrc" ;;
    *) echo "$HOME/.profile" ;;
  esac
}

ensure_path() {
  PROFILE="$1"
  LINE='export PATH="$HOME/.local/bin:$PATH"'

  mkdir -p "$(dirname "$PROFILE")"
  touch "$PROFILE"

  if grep -F "$BIN_DIR" "$PROFILE" >/dev/null 2>&1; then
    return
  fi

  {
    printf "\n# Added by wat installer\n"
    printf "%s\n" "$LINE"
  } >> "$PROFILE"

  echo "added $BIN_DIR to PATH in $PROFILE"
}

need_cmd tar
need_cmd mktemp
need_cmd install

if [ "$(uname -s)" != "Linux" ]; then
  echo "error: Linux only. Use install-macos.sh on macOS." >&2
  exit 1
fi

arch="$(uname -m)"
case "$arch" in
  x86_64|amd64) target="x86_64-unknown-linux-musl" ;;
  *)
    echo "error: unsupported architecture: $arch" >&2
    exit 1
    ;;
esac

archive="wat-${target}.tar.gz"
url="https://github.com/${REPO}/releases/latest/download/${archive}"

tmpdir="$(mktemp -d)"
trap 'rm -rf "$tmpdir"' EXIT INT TERM

mkdir -p "$BIN_DIR"

echo "installing wat from $url"
download "$url" "$tmpdir/$archive"

tar -xzf "$tmpdir/$archive" -C "$tmpdir"
install "$tmpdir/wat" "$BIN_DIR/wat"

echo "wat installed to $BIN_DIR/wat"

PROFILE="$(detect_profile)"

case ":$PATH:" in
  *":$BIN_DIR:"*) ;;
  *)
    ensure_path "$PROFILE"
    echo "restart your shell or run:"
    echo "  export PATH=\"$BIN_DIR:\$PATH\""
    ;;
esac
