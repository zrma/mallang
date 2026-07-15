#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

compiler="target/debug/mlg"
compiler_set=0
check_release_archive=1
for argument in "$@"; do
  case "$argument" in
    --skip-release-archive)
      check_release_archive=0
      ;;
    -*)
      echo "usage: scripts/check-v08-reproducibility.sh [--skip-release-archive] [compiler]" >&2
      exit 2
      ;;
    *)
      if [[ "$compiler_set" -eq 1 ]]; then
        echo "usage: scripts/check-v08-reproducibility.sh [--skip-release-archive] [compiler]" >&2
        exit 2
      fi
      compiler="$argument"
      compiler_set=1
      ;;
  esac
done
if [[ ! -x "$compiler" ]]; then
  echo "compiler is not executable: $compiler" >&2
  exit 1
fi

python3 scripts/measure-v08-baseline.py \
  --check-baseline docs/baselines/v0.8-performance.json

work="target/mallang/v08-reproducibility"
rm -rf "$work"
mkdir -p "$work"

check_generated_c() {
  local label="$1"
  local input="$2"
  local generated_c="$3"

  "$compiler" build "$input" -o "$work/$label-first" >/dev/null
  cp "$generated_c" "$work/$label-first.c"
  "$compiler" build "$input" -o "$work/$label-second" >/dev/null
  if ! cmp -s "$work/$label-first.c" "$generated_c"; then
    echo "repeated generated C differs for $input" >&2
    exit 1
  fi
}

check_generated_c \
  minimal-standalone \
  examples/first.mlg \
  target/mallang/first.c
check_generated_c \
  cleanup-heavy-standalone \
  examples/full-expression-cleanup.mlg \
  target/mallang/full-expression-cleanup.c
check_generated_c \
  local-dependency-project \
  examples/projects/local-deps/app \
  examples/projects/local-deps/app/target/mallang/pathapp.c
check_generated_c \
  standard-library-cli \
  examples/projects/textstats \
  examples/projects/textstats/target/mallang/textstats.c

if [[ "$check_release_archive" -eq 1 ]]; then
  scripts/check-release-artifacts.sh
fi

if [[ "$check_release_archive" -eq 1 ]]; then
  echo "v0.8 generated C and release archive reproducibility passed"
else
  echo "v0.8 generated C reproducibility passed; release archive covered by canonical acceptance"
fi
