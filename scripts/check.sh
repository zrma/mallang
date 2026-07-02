#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

if command -v cargo >/dev/null 2>&1; then
  CARGO="cargo"
else
  TOOLCHAIN_BIN="$HOME/.rustup/toolchains/stable-aarch64-apple-darwin/bin"
  if [[ ! -x "$TOOLCHAIN_BIN/cargo" ]]; then
    echo "cargo not found and fallback toolchain missing: $TOOLCHAIN_BIN/cargo" >&2
    exit 1
  fi
  export PATH="$TOOLCHAIN_BIN:$PATH"
  CARGO="$TOOLCHAIN_BIN/cargo"
fi

"$CARGO" fmt --all --check
"$CARGO" test --workspace
"$CARGO" clippy --workspace --all-targets -- -D warnings
"$CARGO" run --bin mlg -- examples/hello.mlg >/dev/null
