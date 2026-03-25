#!/usr/bin/env sh
set -eu

REPO="${WAT_INSTALL_REPO:-howeg/wat}"
VERSION="${WAT_INSTALL_VERSION:-latest}"
BIN_DIR="${WAT_INSTALL_BIN:-$HOME/.local/bin}"
PROFILE="${WAT_INSTALL_PROFILE:-}"
SET_PATH="${WAT_INSTALL_SET_PATH:-1}"

need_cmd() {
  if ! command -v "$1" >/dev/null 2>&1; then
    echo "error: missing required command: $1" >&2
    exit 1
  fi
}

detect_target() {
  os="$(uname -s)"
  arch="$(uname -m)"

  case "$os" in
    Linux) platform="unknown-linux-musl" ;;
    Darwin) platform="apple-darwin" ;;
    *)
      echo "error: unsupported operating system: $os" >&2
      exit 1
      ;;
  esac

  case "$arch" in
    x86_64|amd64) cpu="x86_64" ;;
    arm64|aarch64) cpu="aarch64" ;;
    *)
      echo "error: unsupported architecture: $arch" >&2
      exit 1
      ;;
  esac

  printf "%s-%s" "$cpu" "$platform"
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

pick_profile() {
  if [ -n "$PROFILE" ]; then
    printf "%s" "$PROFILE"
    return
  fi

  for candidate in "$HOME/.zshrc" "$HOME/.bashrc" "$HOME/.profile"; do
    if [ -f "$candidate" ]; then
      printf "%s" "$candidate"
      return
    fi
  done

  printf "%s" "$HOME/.profile"
}

ensure_path_line() {
  profile_path="$1"
  export_line="export PATH=\"$BIN_DIR:\$PATH\""

  mkdir -p "$(dirname "$profile_path")"
  touch "$profile_path"

  if grep -F "$export_line" "$profile_path" >/dev/null 2>&1; then
    return
  fi

  {
    printf "\n"
    printf "# Added by wat installer\n"
    printf "%s\n" "$export_line"
  } >> "$profile_path"
}

need_cmd tar
need_cmd mktemp
need_cmd install
target="$(detect_target)"
archive="wat-${target}.tar.gz"

case "$VERSION" in
  latest) url="https://github.com/${REPO}/releases/latest/download/${archive}" ;;
  *) url="https://github.com/${REPO}/releases/download/${VERSION}/${archive}" ;;
esac

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
    if [ "$SET_PATH" = "1" ]; then
      profile_path="$(pick_profile)"
      ensure_path_line "$profile_path"
      echo "added $BIN_DIR to PATH in $profile_path"
      echo "restart your shell or run: export PATH=\"$BIN_DIR:\$PATH\""
    else
      echo "add $BIN_DIR to PATH to run wat from any shell"
    fi
    ;;
esac
