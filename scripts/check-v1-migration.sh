#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

COMPILER="${1:-target/debug/mlg}"
if [[ ! -x "$COMPILER" ]]; then
  echo "migration check requires an executable compiler: $COMPILER" >&2
  exit 1
fi

OUT="target/mallang/v1-migration"
mkdir -p "$OUT"

expect_rejection() {
  local label="$1"
  local fixture="$2"
  local expected="$3"
  local stdout="$OUT/${label}.stdout"
  local stderr="$OUT/${label}.stderr"

  if "$COMPILER" check "$fixture" >"$stdout" 2>"$stderr"; then
    echo "migration rejection failed: $fixture was accepted" >&2
    exit 1
  fi
  if [[ -s "$stdout" ]] || ! grep -Fq "$expected" "$stderr" || ! grep -Fq "$fixture:" "$stderr"; then
    echo "migration rejection diagnostic mismatch: $fixture" >&2
    cat "$stdout" >&2
    cat "$stderr" >&2
    exit 1
  fi
}

canonical="tests/fixtures/v1-migration/canonical-borrow-and-range.mlg"
"$COMPILER" check "$canonical" >/dev/null
"$COMPILER" build "$canonical" -o "$OUT/canonical" >/dev/null
output="$("$OUT/canonical")"
if [[ "$output" != $'kim\nlee\na\nb' ]]; then
  echo "canonical migration fixture output mismatch: got '$output'" >&2
  exit 1
fi

expect_rejection \
  suffix-read \
  tests/fixtures/v1-migration/legacy-suffix-read.mlg \
  'expected `)` after function parameters'
expect_rejection \
  suffix-mut \
  tests/fixtures/v1-migration/legacy-suffix-mut.mlg \
  'expected type name'
expect_rejection \
  call-in \
  tests/fixtures/v1-migration/legacy-call-in.mlg \
  'expected `)` after call arguments'
expect_rejection \
  range-con \
  tests/fixtures/v1-migration/legacy-range-con.mlg \
  'by-reference range value bindings are not supported'

echo "v1 canonical borrow/range migration acceptance passed"
