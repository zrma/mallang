#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

if command -v cargo >/dev/null 2>&1; then
  CARGO=(cargo)
elif command -v rustup >/dev/null 2>&1; then
  CARGO=(rustup run stable cargo)
else
  echo "self-hosting lexer check failed: cargo is required" >&2
  exit 1
fi

if command -v rustc >/dev/null 2>&1; then
  RUSTC=(rustc)
elif command -v rustup >/dev/null 2>&1; then
  RUSTC=(rustup run stable rustc)
else
  echo "self-hosting lexer check failed: rustc is required" >&2
  exit 1
fi

CLANG_BIN="${CLANG:-clang}"
command -v "$CLANG_BIN" >/dev/null 2>&1 || {
  echo "self-hosting lexer check failed: clang is required" >&2
  exit 1
}

WORK="target/mallang/self-hosting/b1-lexer"
STAGE0="target/debug/mlg"
STAGE1="$WORK/bootstrap-frontend"
ORACLE="$WORK/bootstrap-frontend-oracle"
PROJECT="bootstrap/compiler"
GENERATED_C="$PROJECT/target/mallang/bootstrap_compiler.c"
FIXTURES="$PROJECT/fixtures/lexer"
mkdir -p "$WORK"

"${CARGO[@]}" build --locked --quiet --lib --bin mlg
"$STAGE0" fmt --check "$PROJECT"
"$STAGE0" check "$PROJECT" >/dev/null
"$STAGE0" test "$PROJECT" >/dev/null

"$STAGE0" build "$PROJECT" -o "$STAGE1" >/dev/null
cp "$GENERATED_C" "$WORK/bootstrap-frontend-first.c"
"$STAGE0" build "$PROJECT" -o "$STAGE1" >/dev/null
if ! cmp -s "$WORK/bootstrap-frontend-first.c" "$GENERATED_C"; then
  echo "self-hosting lexer check failed: Stage0 generated non-deterministic C" >&2
  diff -u "$WORK/bootstrap-frontend-first.c" "$GENERATED_C" >&2 || true
  exit 1
fi

"${RUSTC[@]}" \
  --edition=2021 \
  tools/bootstrap-frontend-oracle.rs \
  --extern mallang=target/debug/libmallang.rlib \
  -L dependency=target/debug/deps \
  -o "$ORACLE"

GENERATED_C_ABS="$(cd "$(dirname "$GENERATED_C")" && pwd)/$(basename "$GENERATED_C")"
cat >"$WORK/accounting.c" <<EOF
#define main mallang_bootstrap_frontend_main
#include "$GENERATED_C_ABS"
#undef main

int main(int argc, char **argv) {
    if (mallang_live_allocation_count() != 0) {
        fprintf(stderr, "bootstrap frontend accounting did not start at zero\n");
        return 2;
    }
    if (mallang_bootstrap_frontend_main(argc, argv) != 0) {
        fprintf(stderr, "bootstrap frontend returned a non-zero status\n");
        return 3;
    }
    if (mallang_live_allocation_count() != 0) {
        fprintf(stderr, "bootstrap frontend leaked compiler-owned allocations\n");
        return 4;
    }
    return 0;
}
EOF

COMMON_FLAGS=(-std=c11 -Wall -Wextra -Werror -pedantic)
"$CLANG_BIN" "${COMMON_FLAGS[@]}" "$WORK/accounting.c" -o "$WORK/accounting"
"$CLANG_BIN" \
  "${COMMON_FLAGS[@]}" \
  -fsanitize=address,undefined \
  -fno-omit-frame-pointer \
  "$WORK/accounting.c" \
  -o "$WORK/accounting-san"

for fixture in "$FIXTURES"/*.mlg; do
  stem="$(basename "$fixture" .mlg)"
  oracle_output="$WORK/$stem.oracle"
  stage1_output="$WORK/$stem.stage1"
  strict_output="$WORK/$stem.strict"
  sanitizer_output="$WORK/$stem.sanitizer"

  "$ORACLE" "$fixture" >"$oracle_output"
  "$STAGE1" "$fixture" >"$stage1_output"
  "$WORK/accounting" "$fixture" >"$strict_output" 2>"$WORK/$stem.strict.stderr"
  "$WORK/accounting-san" "$fixture" >"$sanitizer_output" 2>"$WORK/$stem.sanitizer.stderr"

  for actual in "$stage1_output" "$strict_output" "$sanitizer_output"; do
    if ! cmp -s "$oracle_output" "$actual"; then
      echo "self-hosting lexer differential mismatch: $stem" >&2
      diff -u "$oracle_output" "$actual" >&2 || true
      exit 1
    fi
  done
  if [[ -s "$WORK/$stem.strict.stderr" || -s "$WORK/$stem.sanitizer.stderr" ]]; then
    echo "self-hosting lexer runtime emitted stderr: $stem" >&2
    cat "$WORK/$stem.strict.stderr" "$WORK/$stem.sanitizer.stderr" >&2
    exit 1
  fi
done

for regression in append-match append-match-loop; do
  fixture="tests/fixtures/self-hosting/$regression.mlg"
  binary="$WORK/$regression"
  "$STAGE0" build "$fixture" -o "$binary" >/dev/null
  "$CLANG_BIN" \
    "${COMMON_FLAGS[@]}" \
    -fsanitize=address,undefined \
    -fno-omit-frame-pointer \
    "target/mallang/$regression.c" \
    -o "$binary-san"
  "$binary" >"$WORK/$regression.stdout"
  "$binary-san" >"$WORK/$regression.sanitizer.stdout" 2>"$WORK/$regression.sanitizer.stderr"
  if ! cmp -s "$WORK/$regression.stdout" "$WORK/$regression.sanitizer.stdout" || \
    [[ -s "$WORK/$regression.sanitizer.stderr" ]]; then
    echo "self-hosting lexer cleanup regression failed: $regression" >&2
    cat "$WORK/$regression.sanitizer.stderr" >&2
    exit 1
  fi
done

if [[ "$(cat "$WORK/append-match.stdout")" != "2" ]] || \
  [[ "$(cat "$WORK/append-match-loop.stdout")" != "1" ]]; then
  echo "self-hosting lexer cleanup regression output mismatch" >&2
  exit 1
fi

echo "self-hosting B1 lexer differential, determinism, ownership, and sanitizer gate passed"
