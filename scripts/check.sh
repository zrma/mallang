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
"$CARGO" run --bin mlg -- parse examples/first.mlg >/dev/null
"$CARGO" run --bin mlg -- check examples/first.mlg >/dev/null
"$CARGO" run --bin mlg -- build examples/first.mlg -o target/mallang/first >/dev/null
first_output="$(target/mallang/first)"
if [[ "$first_output" != "30" ]]; then
  echo "first native build smoke failed: expected 30, got '$first_output'" >&2
  exit 1
fi
"$CARGO" run --bin mlg -- check examples/if.mlg >/dev/null
"$CARGO" run --bin mlg -- build examples/if.mlg -o target/mallang/if >/dev/null
if_output="$(target/mallang/if)"
if [[ "$if_output" != "pass" ]]; then
  echo "if native build smoke failed: expected pass, got '$if_output'" >&2
  exit 1
fi
