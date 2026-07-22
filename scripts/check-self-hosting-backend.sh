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
FIXTURES=(scalars owned-control)
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

for name in "${FIXTURES[@]}"; do
  fixture="$PROJECT/fixtures/backend/$name.mlg"
  oracle_c="target/mallang/$name.c"
  stage1_c="$WORK/$name.stage1.c"
  stage1_c_second="$WORK/$name.stage1.second.c"

  "$STAGE0" build "$fixture" -o "$WORK/$name.stage0" >/dev/null
  "$STAGE1" c "$fixture" >"$stage1_c"
  "$STAGE1" c "$fixture" >"$stage1_c_second"

  if ! cmp -s "$oracle_c" "$stage1_c"; then
    echo "self-hosting backend generated C differs from Stage0: $name" >&2
    diff -u "$oracle_c" "$stage1_c" >&2 || true
    exit 1
  fi
  if ! cmp -s "$stage1_c" "$stage1_c_second"; then
    echo "self-hosting backend generated C is not deterministic: $name" >&2
    exit 1
  fi

  "$CLANG_BIN" "${OPTIMIZED_FLAGS[@]}" "$stage1_c" -o "$WORK/$name.stage1"
  "$CLANG_BIN" "${SANITIZER_FLAGS[@]}" "$stage1_c" -o "$WORK/$name.stage1-san"

  generated_c_abs="$(cd "$(dirname "$stage1_c")" && pwd)/$(basename "$stage1_c")"
  cat >"$WORK/$name-accounting.c" <<EOF
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
    "$WORK/$name-accounting.c" -o "$WORK/$name-accounting"
  "$CLANG_BIN" "${SANITIZER_FLAGS[@]}" \
    "$WORK/$name-accounting.c" -o "$WORK/$name-accounting-san"

  "$WORK/$name.stage0" >"$WORK/$name.stage0.stdout"
  "$WORK/$name.stage1" >"$WORK/$name.stage1.stdout"
  "$WORK/$name.stage1-san" >"$WORK/$name.stage1-san.stdout" \
    2>"$WORK/$name.stage1-san.stderr"
  "$WORK/$name-accounting" >"$WORK/$name-accounting.stdout" \
    2>"$WORK/$name-accounting.stderr"
  "$WORK/$name-accounting-san" >"$WORK/$name-accounting-san.stdout" \
    2>"$WORK/$name-accounting-san.stderr"

  for output in \
    "$WORK/$name.stage1.stdout" \
    "$WORK/$name.stage1-san.stdout" \
    "$WORK/$name-accounting.stdout" \
    "$WORK/$name-accounting-san.stdout"; do
    if ! cmp -s "$WORK/$name.stage0.stdout" "$output"; then
      echo "self-hosting backend native output mismatch: $output" >&2
      exit 1
    fi
  done

  expected=""
  case "$name" in
    scalars)
      expected=$'30\ntrue'
      ;;
    owned-control)
      expected=$'ready\nmiddle\nready\nready\nequal\ndifferent\n말랑'
      ;;
    *)
      echo "self-hosting backend fixture has no expected output: $name" >&2
      exit 1
      ;;
  esac
  if [[ "$(cat "$WORK/$name.stage0.stdout")" != "$expected" ]]; then
    echo "self-hosting backend fixture output mismatch: $name" >&2
    exit 1
  fi
  if [[ -s "$WORK/$name.stage1-san.stderr" || \
        -s "$WORK/$name-accounting.stderr" || \
        -s "$WORK/$name-accounting-san.stderr" ]]; then
    echo "self-hosting backend runtime emitted unexpected stderr: $name" >&2
    cat \
      "$WORK/$name.stage1-san.stderr" \
      "$WORK/$name-accounting.stderr" \
      "$WORK/$name-accounting-san.stderr" >&2
    exit 1
  fi
done

echo "self-hosting B3 backend gate passed: fixtures=${#FIXTURES[@]} elapsed=$((SECONDS - started))s"
