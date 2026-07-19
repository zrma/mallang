#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

usage() {
  echo "usage: scripts/check-self-hosting-backend.sh [--assume-bootstrap]" >&2
}

ASSUME_BOOTSTRAP=false
if [[ $# -gt 0 ]]; then
  case "$1" in
    --assume-bootstrap)
      [[ $# -eq 1 ]] || {
        usage
        exit 2
      }
      ASSUME_BOOTSTRAP=true
      ;;
    -h|--help)
      [[ $# -eq 1 ]] || {
        usage
        exit 2
      }
      usage
      exit 0
      ;;
    *)
      usage
      exit 2
      ;;
  esac
fi

if command -v cargo >/dev/null 2>&1; then
  CARGO=(cargo)
elif command -v rustup >/dev/null 2>&1; then
  CARGO=(rustup run stable cargo)
else
  echo "self-hosting backend check failed: cargo is required" >&2
  exit 1
fi

CLANG_BIN="${CLANG:-clang}"
command -v "$CLANG_BIN" >/dev/null 2>&1 || {
  echo "self-hosting backend check failed: clang is required" >&2
  exit 1
}

WORK="target/mallang/self-hosting/b3-backend"
STAGE0="target/debug/mlg"
STAGE1="target/mallang/self-hosting/b1-lexer/bootstrap-frontend"
PROJECT="bootstrap/compiler"
FIXTURE="$PROJECT/fixtures/backend/scalars.mlg"
ORACLE_C="target/mallang/scalars.c"
STAGE1_C="$WORK/scalars.stage1.c"
STAGE1_C_SECOND="$WORK/scalars.stage1.second.c"
OPTIMIZED_FLAGS=(-std=c11 -O2 -Wall -Wextra -Werror -pedantic)
SANITIZER_FLAGS=(
  -std=c11
  -O1
  -Wall
  -Wextra
  -Werror
  -pedantic
  "-fsanitize=address,undefined"
  -fno-omit-frame-pointer
)

mkdir -p "$WORK"
started=$SECONDS

if [[ "$ASSUME_BOOTSTRAP" == false ]]; then
  "${CARGO[@]}" build --locked --quiet --lib --bin mlg
  "$STAGE0" fmt --check "$PROJECT"
  "$STAGE0" check "$PROJECT" >/dev/null
  "$STAGE0" build "$PROJECT" -o "$WORK/bootstrap-compiler" >/dev/null
  "$CLANG_BIN" "${OPTIMIZED_FLAGS[@]}" \
    "$PROJECT/target/mallang/bootstrap_compiler.c" -o "$STAGE1"
elif [[ ! -x "$STAGE1" ]]; then
  echo "self-hosting backend check failed: assumed Stage1 compiler is missing" >&2
  exit 1
fi

"$STAGE0" build "$FIXTURE" -o "$WORK/scalars.stage0" >/dev/null
"$STAGE1" c "$FIXTURE" >"$STAGE1_C"
"$STAGE1" c "$FIXTURE" >"$STAGE1_C_SECOND"

if ! cmp -s "$ORACLE_C" "$STAGE1_C"; then
  echo "self-hosting backend generated C differs from Stage0" >&2
  diff -u "$ORACLE_C" "$STAGE1_C" >&2 || true
  exit 1
fi
if ! cmp -s "$STAGE1_C" "$STAGE1_C_SECOND"; then
  echo "self-hosting backend generated C is not deterministic" >&2
  exit 1
fi

"$CLANG_BIN" "${OPTIMIZED_FLAGS[@]}" "$STAGE1_C" -o "$WORK/scalars.stage1"
"$CLANG_BIN" "${SANITIZER_FLAGS[@]}" "$STAGE1_C" -o "$WORK/scalars.stage1-san"

generated_c_abs="$(cd "$(dirname "$STAGE1_C")" && pwd)/$(basename "$STAGE1_C")"
cat >"$WORK/scalars-accounting.c" <<EOF
#define main mallang_fixture_main
#include "$generated_c_abs"
#undef main

int main(void) {
    if (mallang_live_allocation_count() != 0) {
        fprintf(stderr, "self-hosting backend accounting did not start at zero\n");
        return 2;
    }
    if (mallang_fixture_main() != 0) {
        fprintf(stderr, "self-hosting backend fixture returned a non-zero status\n");
        return 3;
    }
    if (mallang_live_allocation_count() != 0) {
        fprintf(stderr, "self-hosting backend fixture leaked allocations\n");
        return 4;
    }
    return 0;
}
EOF

"$CLANG_BIN" "${OPTIMIZED_FLAGS[@]}" \
  "$WORK/scalars-accounting.c" -o "$WORK/scalars-accounting"
"$CLANG_BIN" "${SANITIZER_FLAGS[@]}" \
  "$WORK/scalars-accounting.c" -o "$WORK/scalars-accounting-san"

"$WORK/scalars.stage0" >"$WORK/scalars.stage0.stdout"
"$WORK/scalars.stage1" >"$WORK/scalars.stage1.stdout"
"$WORK/scalars.stage1-san" >"$WORK/scalars.stage1-san.stdout" \
  2>"$WORK/scalars.stage1-san.stderr"
"$WORK/scalars-accounting" >"$WORK/scalars-accounting.stdout" \
  2>"$WORK/scalars-accounting.stderr"
"$WORK/scalars-accounting-san" >"$WORK/scalars-accounting-san.stdout" \
  2>"$WORK/scalars-accounting-san.stderr"

for output in \
  "$WORK/scalars.stage1.stdout" \
  "$WORK/scalars.stage1-san.stdout" \
  "$WORK/scalars-accounting.stdout" \
  "$WORK/scalars-accounting-san.stdout"; do
  if ! cmp -s "$WORK/scalars.stage0.stdout" "$output"; then
    echo "self-hosting backend native output mismatch: $output" >&2
    exit 1
  fi
done

if [[ "$(cat "$WORK/scalars.stage0.stdout")" != $'30\ntrue' ]]; then
  echo "self-hosting backend fixture output mismatch" >&2
  exit 1
fi
if [[ -s "$WORK/scalars.stage1-san.stderr" || \
      -s "$WORK/scalars-accounting.stderr" || \
      -s "$WORK/scalars-accounting-san.stderr" ]]; then
  echo "self-hosting backend runtime emitted unexpected stderr" >&2
  cat \
    "$WORK/scalars.stage1-san.stderr" \
    "$WORK/scalars-accounting.stderr" \
    "$WORK/scalars-accounting-san.stderr" >&2
  exit 1
fi

echo "self-hosting B3 scalar backend gate passed: elapsed=$((SECONDS - started))s"
