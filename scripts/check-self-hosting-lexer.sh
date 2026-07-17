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
PARSER_FIXTURES="$PROJECT/fixtures/parser"
SEMANTIC_FIXTURES="$PROJECT/fixtures/semantic"
IR_FIXTURES="$PROJECT/fixtures/ir"
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
        fprintf(
            stderr,
            "bootstrap frontend leaked compiler-owned allocations: %lld\n",
            (long long)mallang_live_allocation_count()
        );
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

compare_parser_fixture() {
  local fixture="$1"
  local stem="$2"
  oracle_output="$WORK/$stem.oracle"
  stage1_output="$WORK/$stem.stage1"
  strict_output="$WORK/$stem.strict"
  sanitizer_output="$WORK/$stem.sanitizer"

  "$ORACLE" parse "$fixture" >"$oracle_output"
  "$STAGE1" parse "$fixture" >"$stage1_output"
  "$WORK/accounting" parse "$fixture" >"$strict_output" 2>"$WORK/$stem.strict.stderr"
  "$WORK/accounting-san" parse "$fixture" >"$sanitizer_output" 2>"$WORK/$stem.sanitizer.stderr"

  for actual in "$stage1_output" "$strict_output" "$sanitizer_output"; do
    if ! cmp -s "$oracle_output" "$actual"; then
      echo "self-hosting parser differential mismatch: $stem" >&2
      diff -u "$oracle_output" "$actual" >&2 || true
      exit 1
    fi
  done
  if [[ -s "$WORK/$stem.strict.stderr" || -s "$WORK/$stem.sanitizer.stderr" ]]; then
    echo "self-hosting parser runtime emitted stderr: $stem" >&2
    cat "$WORK/$stem.strict.stderr" "$WORK/$stem.sanitizer.stderr" >&2
    exit 1
  fi
}

for fixture in "$PARSER_FIXTURES"/*.mlg; do
  compare_parser_fixture "$fixture" "parser-$(basename "$fixture" .mlg)"
done

compare_semantic_fixture() {
  local fixture="$1"
  local stem="$2"
  oracle_output="$WORK/$stem.oracle"
  stage1_output="$WORK/$stem.stage1"
  strict_output="$WORK/$stem.strict"
  sanitizer_output="$WORK/$stem.sanitizer"

  "$ORACLE" check "$fixture" >"$oracle_output"
  "$STAGE1" check "$fixture" >"$stage1_output"
  "$WORK/accounting" check "$fixture" >"$strict_output" 2>"$WORK/$stem.strict.stderr"
  "$WORK/accounting-san" check "$fixture" >"$sanitizer_output" 2>"$WORK/$stem.sanitizer.stderr"

  for actual in "$stage1_output" "$strict_output" "$sanitizer_output"; do
    if ! cmp -s "$oracle_output" "$actual"; then
      echo "self-hosting semantic differential mismatch: $stem" >&2
      diff -u "$oracle_output" "$actual" >&2 || true
      exit 1
    fi
  done
  if [[ -s "$WORK/$stem.strict.stderr" || -s "$WORK/$stem.sanitizer.stderr" ]]; then
    echo "self-hosting semantic runtime emitted stderr: $stem" >&2
    cat "$WORK/$stem.strict.stderr" "$WORK/$stem.sanitizer.stderr" >&2
    exit 1
  fi
}

for fixture in "$SEMANTIC_FIXTURES"/*.mlg; do
  compare_semantic_fixture "$fixture" "semantic-$(basename "$fixture" .mlg)"
done

compare_ir_fixture() {
  local fixture="$1"
  local stem="$2"
  oracle_output="$WORK/$stem.oracle"
  stage1_output="$WORK/$stem.stage1"
  strict_output="$WORK/$stem.strict"
  sanitizer_output="$WORK/$stem.sanitizer"

  "$ORACLE" ir "$fixture" >"$oracle_output"
  "$STAGE1" ir "$fixture" >"$stage1_output"
  "$WORK/accounting" ir "$fixture" >"$strict_output" 2>"$WORK/$stem.strict.stderr"
  "$WORK/accounting-san" ir "$fixture" >"$sanitizer_output" 2>"$WORK/$stem.sanitizer.stderr"

  for actual in "$stage1_output" "$strict_output" "$sanitizer_output"; do
    if ! cmp -s "$oracle_output" "$actual"; then
      echo "self-hosting typed IR differential mismatch: $stem" >&2
      diff -u "$oracle_output" "$actual" >&2 || true
      exit 1
    fi
  done
  if [[ -s "$WORK/$stem.strict.stderr" || -s "$WORK/$stem.sanitizer.stderr" ]]; then
    echo "self-hosting typed IR runtime emitted stderr: $stem" >&2
    cat "$WORK/$stem.strict.stderr" "$WORK/$stem.sanitizer.stderr" >&2
    exit 1
  fi
}

for fixture in "$IR_FIXTURES"/*.mlg; do
  compare_ir_fixture "$fixture" "ir-$(basename "$fixture" .mlg)"
done

PARSER_CORPUS_LIST="$WORK/parser-corpus.list"
find \
  bootstrap/compiler/src \
  bootstrap/compiler/tests \
  examples \
  tests/fixtures \
  -type f -name '*.mlg' -print | LC_ALL=C sort >"$PARSER_CORPUS_LIST"

parser_corpus_count=0
while IFS= read -r fixture; do
  stem="parser-corpus-$(printf '%04d' "$parser_corpus_count")"
  compare_parser_fixture "$fixture" "$stem"
  parser_corpus_count=$((parser_corpus_count + 1))
done <"$PARSER_CORPUS_LIST"

if ((parser_corpus_count < 150)); then
  echo "self-hosting parser corpus unexpectedly small: $parser_corpus_count" >&2
  exit 1
fi

for regression in append-match append-match-loop match-arm-return-temp string-equality-temporaries; do
  fixture="tests/fixtures/self-hosting/$regression.mlg"
  binary="$WORK/$regression"
  "$STAGE0" build "$fixture" -o "$binary" >/dev/null
  generated_c="target/mallang/$regression.c"
  generated_c_abs="$(cd "$(dirname "$generated_c")" && pwd)/$(basename "$generated_c")"
  accounting_source="$WORK/$regression-accounting.c"
  cat >"$accounting_source" <<EOF
#define main mallang_fixture_main
#include "$generated_c_abs"
#undef main

int main(void) {
    if (mallang_live_allocation_count() != 0) {
        fprintf(stderr, "self-hosting regression accounting did not start at zero\n");
        return 2;
    }
    if (mallang_fixture_main() != 0) {
        fprintf(stderr, "self-hosting regression returned a non-zero status\n");
        return 3;
    }
    if (mallang_live_allocation_count() != 0) {
        fprintf(
            stderr,
            "self-hosting regression leaked compiler-owned allocations: %lld\n",
            (long long)mallang_live_allocation_count()
        );
        return 4;
    }
    return 0;
}
EOF
  "$CLANG_BIN" "${COMMON_FLAGS[@]}" "$accounting_source" -o "$binary-accounting"
  "$CLANG_BIN" \
    "${COMMON_FLAGS[@]}" \
    -fsanitize=address,undefined \
    -fno-omit-frame-pointer \
    "$accounting_source" \
    -o "$binary-san"
  "$binary" >"$WORK/$regression.stdout"
  "$binary-accounting" >"$WORK/$regression.accounting.stdout" \
    2>"$WORK/$regression.accounting.stderr"
  "$binary-san" >"$WORK/$regression.sanitizer.stdout" 2>"$WORK/$regression.sanitizer.stderr"
  if ! cmp -s "$WORK/$regression.stdout" "$WORK/$regression.accounting.stdout" || \
    ! cmp -s "$WORK/$regression.stdout" "$WORK/$regression.sanitizer.stdout" || \
    [[ -s "$WORK/$regression.accounting.stderr" || \
      -s "$WORK/$regression.sanitizer.stderr" ]]; then
    echo "self-hosting lexer cleanup regression failed: $regression" >&2
    cat "$WORK/$regression.accounting.stderr" "$WORK/$regression.sanitizer.stderr" >&2
    exit 1
  fi
done

if [[ "$(cat "$WORK/append-match.stdout")" != "2" ]] || \
  [[ "$(cat "$WORK/append-match-loop.stdout")" != "1" ]] || \
  [[ "$(cat "$WORK/match-arm-return-temp.stdout")" != "7" ]] || \
  [[ "$(cat "$WORK/string-equality-temporaries.stdout")" != $'true\ntrue' ]]; then
  echo "self-hosting lexer cleanup regression output mismatch" >&2
  exit 1
fi

echo "self-hosting B2d1a composite literal differential, B1 frontend, determinism, accounting, and sanitizer gate passed: parser-corpus=$parser_corpus_count"
