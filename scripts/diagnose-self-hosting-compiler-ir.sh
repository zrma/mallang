#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

usage() {
  cat >&2 <<'EOF'
usage: scripts/diagnose-self-hosting-compiler-ir.sh [--rebuild-bootstrap | --reuse-bootstrap] [--max-diff-lines <count>]

Runs a fresh focused IR gate by default. --rebuild-bootstrap regenerates only
the optimized Stage1 binary, while --reuse-bootstrap uses existing artifacts.
Both alternatives are explicit, non-gating inner-loop shortcuts.
EOF
}

bootstrap_mode=fresh
max_diff_lines=80
while [[ $# -gt 0 ]]; do
  case "$1" in
    --rebuild-bootstrap)
      if [[ "$bootstrap_mode" != "fresh" ]]; then
        usage
        exit 2
      fi
      bootstrap_mode=rebuild
      shift
      ;;
    --reuse-bootstrap)
      if [[ "$bootstrap_mode" != "fresh" ]]; then
        usage
        exit 2
      fi
      bootstrap_mode=reuse
      shift
      ;;
    --max-diff-lines)
      if [[ $# -lt 2 || ! "$2" =~ ^[0-9]+$ ]]; then
        usage
        exit 2
      fi
      max_diff_lines="$2"
      shift 2
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      usage
      exit 2
      ;;
  esac
done

WORK="target/mallang/self-hosting/b1-lexer"
STAGE1="$WORK/bootstrap-frontend"
ORACLE="$WORK/bootstrap-frontend-oracle"
STAGE0="target/debug/mlg"
GENERATED_C="bootstrap/compiler/target/mallang/bootstrap_compiler.c"
ORACLE_OUTPUT="$WORK/compiler-source-ir.oracle"
STAGE1_OUTPUT="$WORK/compiler-source-ir.stage1"

case "$bootstrap_mode" in
  fresh)
    scripts/check-self-hosting-lexer.sh --focus ir
    ;;
  rebuild)
    if [[ ! -x "$STAGE0" || ! -x "$ORACLE" ]]; then
      echo "self-hosting compiler IR diagnosis has no rebuild seed artifacts" >&2
      exit 2
    fi
    echo "self-hosting compiler IR diagnosis is rebuilding non-gating Stage1 artifacts"
    "$STAGE0" build bootstrap/compiler -o "$STAGE1" >/dev/null
    "${CLANG:-clang}" \
      -std=c11 -O2 -Wall -Wextra -Werror -pedantic \
      "$GENERATED_C" -o "$STAGE1"
    ;;
  reuse)
    if [[ ! -x "$STAGE1" || ! -x "$ORACLE" ]]; then
      echo "self-hosting compiler IR diagnosis has no reusable bootstrap artifacts" >&2
      exit 2
    fi
    echo "self-hosting compiler IR diagnosis is reusing non-gating bootstrap artifacts"
    ;;
esac

compiler_sources=()
while IFS= read -r source_path; do
  compiler_sources+=("$source_path")
done < <(find bootstrap/compiler/src -type f -name '*.mlg' -print | LC_ALL=C sort)

started=$SECONDS
"$ORACLE" \
  ir-project 1 bootstrap_compiler bootstrap/compiler/src 0 \
  "${compiler_sources[@]}" >"$ORACLE_OUTPUT"
oracle_elapsed=$((SECONDS - started))

started=$SECONDS
"$STAGE1" \
  ir-project 1 bootstrap_compiler bootstrap/compiler/src 0 \
  "${compiler_sources[@]}" >"$STAGE1_OUTPUT"
stage1_elapsed=$((SECONDS - started))

set +e
scripts/compare-self-hosting-ir.py \
  --max-diff-lines "$max_diff_lines" \
  "$ORACLE_OUTPUT" \
  "$STAGE1_OUTPUT"
status=$?
set -e

echo "self-hosting compiler IR timing: stage0=${oracle_elapsed}s stage1=${stage1_elapsed}s"
exit "$status"
