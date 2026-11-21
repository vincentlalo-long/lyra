#!/usr/bin/env bash
# Enforce the zero-warning policy for rustc diagnostics across every shipped
# feature combination. CI and `make check` both run this file, so a local run
# matches the gate exactly.
set -euo pipefail

cd "$(dirname "${BASH_SOURCE[0]}")/.."

manifest="MusicPlayer/Cargo.toml"
features=(download mpris notify genre update)
log="$(mktemp)"
trap 'rm -f "$log"' EXIT

failed=0
report() {
  if [[ "${GITHUB_ACTIONS:-}" == "true" ]]; then
    printf '::error::%s\n' "$*"
  else
    printf 'error: %s\n' "$*" >&2
  fi
}

check() {
  printf '==> cargo check %s\n' "$*"
  if ! cargo check --manifest-path "$manifest" --all-targets "$@" 2>&1 | tee "$log"; then
    report "cargo check failed: $*"
    failed=1
  elif grep -qE '(^warning: |: warning: )' "$log"; then
    report "rustc warnings in: cargo check $*"
    failed=1
  fi
}

check
check --no-default-features
for feature in "${features[@]}"; do
  check --no-default-features --features "$feature"
done

if ((failed)); then
  printf 'Zero-warning policy violated.\n' >&2
  exit 1
fi
printf 'All feature combinations are warning-free.\n'
