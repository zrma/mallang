#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

usage() {
  cat >&2 <<'EOF'
usage: scripts/build-self-hosted-compiler.sh [--stage0 <path>] [--output <path>]

Builds the internal `mlgc` compiler through the non-recursive
Stage0 -> Stage1 -> fixed Stage2 bootstrap graph. The default output is
target/debug/mlgc, next to the development `mlg` driver.
EOF
}

fail_usage() {
  echo "$1" >&2
  usage
  exit 2
}

stage0=""
output="target/debug/mlgc"
while [[ $# -gt 0 ]]; do
  case "$1" in
    --stage0)
      [[ $# -ge 2 && -n "$2" && "$2" != --* ]] || fail_usage "missing value for --stage0"
      stage0="$2"
      shift 2
      ;;
    --output)
      [[ $# -ge 2 && -n "$2" && "$2" != --* ]] || fail_usage "missing value for --output"
      output="$2"
      shift 2
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      fail_usage "unknown argument: $1"
      ;;
  esac
done

clang_bin="${CLANG:-clang}"
command -v "$clang_bin" >/dev/null 2>&1 || {
  echo "self-hosted compiler build failed: clang is required" >&2
  exit 1
}

if [[ -z "$stage0" ]]; then
  if command -v cargo >/dev/null 2>&1; then
    cargo_command=(cargo)
  elif command -v rustup >/dev/null 2>&1; then
    cargo_command=(rustup run stable cargo)
  else
    echo "self-hosted compiler build failed: cargo is required without --stage0" >&2
    exit 1
  fi
  "${cargo_command[@]}" build --locked --quiet --lib --bin mlg
  stage0="target/debug/mlg"
fi
if [[ ! -x "$stage0" ]]; then
  echo "self-hosted compiler build failed: Stage0 is not executable: $stage0" >&2
  exit 1
fi

project="bootstrap/compiler"
work="target/mallang/self-hosting/b5-bootstrap"
stage1="$work/mlgc.stage1"
stage2="$work/mlgc.stage2"
stage2_c="$work/mlgc.stage2.c"
fixed_c="$work/mlgc.fixed.c"
generated_c="$project/target/mallang/bootstrap_compiler.c"
strict_flags=(-std=c11 -O2 -Wall -Wextra -Werror -pedantic)

mkdir -p "$work" "$(dirname "$output")"
"$stage0" fmt --check "$project"
"$stage0" check "$project" >/dev/null
"$stage0" build "$project" -o "$stage1" >/dev/null
"$clang_bin" "${strict_flags[@]}" "$generated_c" -o "$stage1"

compiler_sources=()
while IFS= read -r source_path; do
  compiler_sources+=("$source_path")
done < <(find "$project/src" -type f -name '*.mlg' -print | LC_ALL=C sort)

"$stage1" c-project \
  1 bootstrap_compiler "$project/src" 0 "${compiler_sources[@]}" \
  >"$stage2_c" 2>"$work/stage1.stderr"
if [[ -s "$work/stage1.stderr" ]]; then
  echo "self-hosted compiler Stage1 emitted stderr" >&2
  cat "$work/stage1.stderr" >&2
  exit 1
fi
"$clang_bin" "${strict_flags[@]}" "$stage2_c" -o "$stage2"

if [[ "$("$stage2" --version)" != "mlgc protocol 1" ]]; then
  echo "self-hosted compiler protocol handshake failed" >&2
  exit 1
fi
"$stage2" c-project \
  1 bootstrap_compiler "$project/src" 0 "${compiler_sources[@]}" \
  >"$fixed_c" 2>"$work/stage2.stderr"
if [[ -s "$work/stage2.stderr" ]]; then
  echo "self-hosted compiler Stage2 emitted stderr" >&2
  cat "$work/stage2.stderr" >&2
  exit 1
fi
if ! cmp -s "$stage2_c" "$fixed_c"; then
  echo "self-hosted compiler build did not reach a Stage1-to-Stage2 fixed point" >&2
  exit 1
fi

temporary_output="${output}.tmp.$$"
trap 'rm -f "$temporary_output"' EXIT
cp "$stage2" "$temporary_output"
chmod 0755 "$temporary_output"
mv -f "$temporary_output" "$output"
trap - EXIT

printf '%s\n' "$output"
