#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

if command -v cargo >/dev/null 2>&1; then
  CARGO=(cargo)
elif command -v rustup >/dev/null 2>&1; then
  CARGO=(rustup run stable cargo)
else
  echo "self-hosting bootstrap check failed: cargo is required" >&2
  exit 1
fi

command -v clang >/dev/null 2>&1 || {
  echo "self-hosting bootstrap check failed: clang is required" >&2
  exit 1
}

WORK="target/mallang/self-hosting/b0"
STAGE0="target/debug/mlg"
PROBE_PROJECT="bootstrap/probe"
PROBE_C="$PROBE_PROJECT/target/mallang/bootstrap_probe.c"
mkdir -p "$WORK"

"${CARGO[@]}" build --locked --quiet --bin mlg
"$STAGE0" fmt --check "$PROBE_PROJECT"
"$STAGE0" check "$PROBE_PROJECT" >/dev/null
"$STAGE0" test "$PROBE_PROJECT" >/dev/null

"$STAGE0" build "$PROBE_PROJECT" -o "$WORK/probe-first" >/dev/null
cp "$PROBE_C" "$WORK/probe-first.c"
"$STAGE0" build "$PROBE_PROJECT" -o "$WORK/probe-second" >/dev/null
cmp "$WORK/probe-first.c" "$PROBE_C"

expected=$'bootstrap-host-ready\nsource_bytes=15\nhas_main=true'
first_output="$("$WORK/probe-first" "$PROBE_PROJECT/fixtures/minimal.mlg")"
second_output="$("$WORK/probe-second" "$PROBE_PROJECT/fixtures/minimal.mlg")"
if [[ "$first_output" != "$expected" || "$second_output" != "$expected" ]]; then
  echo "self-hosting bootstrap check failed: probe output mismatch" >&2
  exit 1
fi

printf 'self-hosting B0 bootstrap check passed: Stage0 built deterministic Mallang probe C\n'
