#!/usr/bin/env sh
set -eu

REPO="howeg/wat"

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

pick_bin_dir() {
  for candidate in "/opt/homebrew/bin" "/usr/local/bin"; do
    if [ -d "$candidate" ] && [ -w "$candidate" ]; then
      printf "%s" "$candidate"
      return
    fi
  done

  printf "%s" "$HOME/.local/bin"
}

pick_profile() {
  for candidate in "$HOME/.zshrc" "$HOME/.bash_profile" "$HOME/.profile"; do
    if [ -f "$candidate" ]; then
      printf "%s" "$candidate"
      return
    fi
  done

  printf "%s" "$HOME/.zshrc"
}

ensure_path_line() {
  profile_path="$1"
  bin_dir="$2"
  export_line="export PATH=\"$bin_dir:\$PATH\""

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

if [ "$(uname -s)" != "Darwin" ]; then
  echo "error: scripts/install-macos.sh is for macOS." >&2
  exit 1
fi

arch="$(uname -m)"
case "$arch" in
  arm64|aarch64) target="aarch64-apple-darwin" ;;
  x86_64|amd64) target="x86_64-apple-darwin" ;;
  *)
    echo "error: unsupported macOS architecture: $arch" >&2
    exit 1
    ;;
esac

bin_dir="$(pick_bin_dir)"
profile_path="$(pick_profile)"
archive="wat-${target}.tar.gz"
url="https://github.com/${REPO}/releases/latest/download/${archive}"
tmpdir="$(mktemp -d)"
trap 'rm -rf "$tmpdir"' EXIT INT TERM

mkdir -p "$bin_dir"
echo "installing wat from ${url}"
download "$url" "$tmpdir/$archive"
tar -xzf "$tmpdir/$archive" -C "$tmpdir"
install "$tmpdir/wat" "$bin_dir/wat"

echo "wat installed to $bin_dir/wat"
case ":$PATH:" in
  *":$bin_dir:"*) ;;
  *)
    ensure_path_line "$profile_path" "$bin_dir"
    echo "added $bin_dir to PATH in $profile_path"
    echo "restart your shell or run: export PATH=\"$bin_dir:\$PATH\""
    ;;
esac
