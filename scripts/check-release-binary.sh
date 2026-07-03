#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

CARGO=(cargo)
if [[ -n "${CARGO_BIN:-}" ]]; then
  CARGO=("$CARGO_BIN")
fi

RELEASE_BIN="target/release/mlg"
SMOKE_BIN="target/mallang/release-binary-first"

"${CARGO[@]}" build --release --bin mlg

crate_version="$(sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml)"
version_output="$("$RELEASE_BIN" --version)"
if [[ "$version_output" != "mlg $crate_version" ]]; then
  echo "release binary version smoke failed: expected mlg $crate_version, got '$version_output'" >&2
  exit 1
fi

help_output="$("$RELEASE_BIN" --help)"
if [[ "$help_output" != *"usage:"* || "$help_output" != *"$RELEASE_BIN check <source-file>"* || "$help_output" != *"$RELEASE_BIN --version"* ]]; then
  echo "release binary help smoke failed" >&2
  echo "$help_output" >&2
  exit 1
fi

check_output="$("$RELEASE_BIN" check examples/first.mlg)"
if [[ "$check_output" != "examples/first.mlg: ok" ]]; then
  echo "release binary check smoke failed: $check_output" >&2
  exit 1
fi

run_command_output="$("$RELEASE_BIN" run examples/first.mlg)"
if [[ "$run_command_output" != "30" ]]; then
  echo "release binary run smoke failed: $run_command_output" >&2
  exit 1
fi

"$RELEASE_BIN" build examples/first.mlg -o "$SMOKE_BIN"
run_output="$("$SMOKE_BIN")"
if [[ "$run_output" != "30" ]]; then
  echo "release binary native build smoke failed: $run_output" >&2
  exit 1
fi

echo "release binary smoke passed"
