#!/usr/bin/env bash
# Install a prebuilt Lyra binary from GitHub Releases.
#
#   curl -fsSL https://raw.githubusercontent.com/vincentlalo-long/lyra/main/scripts/install.sh | bash
#   ... | bash -s -- --minimal
#
# The archive is checked against the SHA-256 in the release's SHA256SUMS
# before it is installed.
set -euo pipefail

repo="vincentlalo-long/lyra"
flavor="full"
tag=""
dir="${LYRA_INSTALL_DIR:-$HOME/.local/bin}"
verify=1

say() { printf '%s\n' "$*"; }
warn() { printf 'warning: %s\n' "$*" >&2; }
die() {
  printf 'error: %s\n' "$*" >&2
  exit 1
}

usage() {
  cat <<'EOF'
Install a prebuilt Lyra binary from GitHub Releases.

Usage: install.sh [options]

  --minimal        lean build with no Python or network dependencies
  --dir DIR        install directory (default: $LYRA_INSTALL_DIR or ~/.local/bin)
  --version TAG    install a specific release tag, e.g. v0.3.4
  --no-verify      skip SHA256SUMS verification
  -h, --help       show this help
EOF
}

while (($#)); do
  case "$1" in
    --minimal) flavor="minimal" ;;
    --dir)
      dir="${2:-}"
      [[ -n "$dir" ]] || die "--dir needs a path"
      shift
      ;;
    --version)
      tag="${2:-}"
      [[ -n "$tag" ]] || die "--version needs a tag such as v0.3.4"
      shift
      ;;
    --no-verify) verify=0 ;;
    -h | --help)
      usage
      exit 0
      ;;
    *) die "unknown option: $1 (try --help)" ;;
  esac
  shift
done

# Expand a leading ~ so quoted paths behave like unquoted ones.
case "$dir" in
  "~/"*) dir="$HOME/${dir#\~/}" ;;
  "~") dir="$HOME" ;;
esac

[[ "$(uname -s)" == "Linux" ]] ||
  die "prebuilt binaries are Linux-only; build from source: https://github.com/$repo#building-from-source"
case "$(uname -m)" in
  x86_64 | amd64) ;;
  *) die "no prebuilt binary for $(uname -m); build from source: https://github.com/$repo#building-from-source" ;;
esac

asset="lyra-linux-x86_64.tar.gz"
[[ "$flavor" == "minimal" ]] && asset="lyra-minimal-linux-x86_64.tar.gz"

if [[ -n "$tag" ]]; then
  base="https://github.com/$repo/releases/download/$tag"
else
  base="https://github.com/$repo/releases/latest/download"
fi

fetch() {
  local url="$1" out="$2"
  if command -v curl >/dev/null 2>&1; then
    curl -fsSL --retry 3 --connect-timeout 15 -o "$out" "$url"
  elif command -v wget >/dev/null 2>&1; then
    wget -qO "$out" "$url"
  else
    die "curl or wget is required"
  fi
}

sha256_of() {
  if command -v sha256sum >/dev/null 2>&1; then
    sha256sum "$1" | awk '{print $1}'
  elif command -v shasum >/dev/null 2>&1; then
    shasum -a 256 "$1" | awk '{print $1}'
  else
    die "sha256sum or shasum is required (or pass --no-verify)"
  fi
}

tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT

say "Downloading $asset ..."
fetch "$base/$asset" "$tmp/$asset"

if ((verify)); then
  if fetch "$base/SHA256SUMS" "$tmp/SHA256SUMS" 2>/dev/null; then
    expected="$(awk -v f="$asset" '$2 == f { print $1; exit }' "$tmp/SHA256SUMS")"
    if [[ -z "$expected" ]]; then
      warn "SHA256SUMS has no entry for $asset; skipping verification"
    elif [[ "$(sha256_of "$tmp/$asset")" == "$expected" ]]; then
      say "Checksum verified."
    else
      die "checksum mismatch for $asset"
    fi
  else
    warn "SHA256SUMS not available; skipping verification"
  fi
fi

mkdir -p "$dir"
tar -xzf "$tmp/$asset" -C "$tmp" lyra
install -m 0755 "$tmp/lyra" "$dir/lyra"
say "Installed $dir/lyra ($flavor${tag:+, $tag})."

case ":$PATH:" in
  *":$dir:"*) ;;
  *) warn "$dir is not in your PATH. Add it with: export PATH=\"$dir:\$PATH\"" ;;
esac

if [[ "$flavor" == "full" ]]; then
  say "Check runtime dependencies with: $dir/lyra --check-deps"
else
  say "Minimal build: only the system audio libraries are required."
fi
