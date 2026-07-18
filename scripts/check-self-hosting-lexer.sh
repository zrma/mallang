#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

MODE="full"
if [[ $# -gt 1 ]] || [[ $# -eq 1 && "$1" != "--fast" ]]; then
  echo "usage: scripts/check-self-hosting-lexer.sh [--fast]" >&2
  exit 2
fi
if [[ $# -eq 1 ]]; then
  MODE="fast"
fi

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
IR_TEST_FIXTURES="$PROJECT/fixtures/ir-test"
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
gate_started=$SECONDS

"${CARGO[@]}" build --locked --quiet --lib --bin mlg
"$STAGE0" fmt --check "$PROJECT"
"$STAGE0" check "$PROJECT" >/dev/null
if [[ "$MODE" == "full" ]]; then
  "$STAGE0" test "$PROJECT" >/dev/null
else
  for test_id in \
    bootstrap_compiler/frontend/lexer::NormalizesKeywordsOperatorsAndPayloads \
    bootstrap_compiler/frontend/parser::RecoversMultipleParserDiagnostics \
    bootstrap_compiler/semantic::ChecksPrintStatementReads \
    bootstrap_compiler/ir::LowersLoopCleanupAtExitAndControlFlow; do
    "$STAGE0" test "$PROJECT" --exact "$test_id" >/dev/null
  done
fi

"$STAGE0" build "$PROJECT" -o "$STAGE1" >/dev/null
cp "$GENERATED_C" "$WORK/bootstrap-frontend-first.c"
"$STAGE0" build "$PROJECT" -o "$STAGE1" >/dev/null
if ! cmp -s "$WORK/bootstrap-frontend-first.c" "$GENERATED_C"; then
  echo "self-hosting lexer check failed: Stage0 generated non-deterministic C" >&2
  diff -u "$WORK/bootstrap-frontend-first.c" "$GENERATED_C" >&2 || true
  exit 1
fi
"$CLANG_BIN" "${OPTIMIZED_FLAGS[@]}" "$GENERATED_C" -o "$STAGE1"

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

"$CLANG_BIN" "${OPTIMIZED_FLAGS[@]}" "$WORK/accounting.c" -o "$WORK/accounting"
"$CLANG_BIN" "${SANITIZER_FLAGS[@]}" "$WORK/accounting.c" -o "$WORK/accounting-san"

compare_fixture() {
  local kind="$1"
  local fixture="$2"
  local stem="$3"
  local profile="$4"
  local label
  local command=""
  local -a actual_outputs=()
  case "$kind" in
    lexer)
      label="lexer"
      ;;
    parser)
      label="parser"
      command="parse"
      ;;
    semantic)
      label="semantic"
      command="check"
      ;;
    ir)
      label="typed IR"
      command="ir"
      ;;
    ir-test)
      label="test typed IR"
      command="ir-test"
      ;;
    *)
      echo "unknown self-hosting differential kind: $kind" >&2
      exit 2
      ;;
  esac

  oracle_output="$WORK/$stem.oracle"
  stage1_output="$WORK/$stem.stage1"
  strict_output="$WORK/$stem.strict"
  sanitizer_output="$WORK/$stem.sanitizer"

  if [[ -n "$command" ]]; then
    "$ORACLE" "$command" "$fixture" >"$oracle_output"
    "$STAGE1" "$command" "$fixture" >"$stage1_output"
  else
    "$ORACLE" "$fixture" >"$oracle_output"
    "$STAGE1" "$fixture" >"$stage1_output"
  fi
  actual_outputs+=("$stage1_output")
  if [[ "$profile" != "stage1" ]]; then
    if [[ -n "$command" ]]; then
      "$WORK/accounting" "$command" "$fixture" \
        >"$strict_output" 2>"$WORK/$stem.strict.stderr"
    else
      "$WORK/accounting" "$fixture" \
        >"$strict_output" 2>"$WORK/$stem.strict.stderr"
    fi
    actual_outputs+=("$strict_output")
  fi
  if [[ "$profile" == "full" ]]; then
    if [[ -n "$command" ]]; then
      "$WORK/accounting-san" "$command" "$fixture" \
        >"$sanitizer_output" 2>"$WORK/$stem.sanitizer.stderr"
    else
      "$WORK/accounting-san" "$fixture" \
        >"$sanitizer_output" 2>"$WORK/$stem.sanitizer.stderr"
    fi
    actual_outputs+=("$sanitizer_output")
  fi

  for actual in "${actual_outputs[@]}"; do
    if ! cmp -s "$oracle_output" "$actual"; then
      echo "self-hosting $label differential mismatch: $stem" >&2
      diff -u "$oracle_output" "$actual" >&2 || true
      exit 1
    fi
  done
  if [[ "$profile" != "stage1" && -s "$WORK/$stem.strict.stderr" ]]; then
    echo "self-hosting $label runtime emitted stderr: $stem" >&2
    cat "$WORK/$stem.strict.stderr" >&2
    exit 1
  fi
  if [[ "$profile" == "full" && -s "$WORK/$stem.sanitizer.stderr" ]]; then
    echo "self-hosting $label runtime emitted stderr: $stem" >&2
    cat "$WORK/$stem.sanitizer.stderr" >&2
    exit 1
  fi
}

fixture_profile="full"
corpus_profile="full"
if [[ "$MODE" == "fast" ]]; then
  fixture_profile="strict"
  corpus_profile="stage1"
fi

for fixture in "$FIXTURES"/*.mlg; do
  compare_fixture lexer "$fixture" "$(basename "$fixture" .mlg)" "$fixture_profile"
done

for fixture in "$PARSER_FIXTURES"/*.mlg; do
  compare_fixture parser "$fixture" "parser-$(basename "$fixture" .mlg)" "$fixture_profile"
done

for fixture in "$SEMANTIC_FIXTURES"/*.mlg; do
  compare_fixture semantic "$fixture" "semantic-$(basename "$fixture" .mlg)" "$fixture_profile"
done

for fixture in "$IR_FIXTURES"/*.mlg; do
  compare_fixture ir "$fixture" "ir-$(basename "$fixture" .mlg)" "$fixture_profile"
done

for fixture in "$IR_TEST_FIXTURES"/*.mlg; do
  compare_fixture ir-test "$fixture" "ir-test-$(basename "$fixture" .mlg)" "$fixture_profile"
done

if [[ "$MODE" == "fast" ]]; then
  compare_fixture lexer "$FIXTURES/all-tokens.mlg" fast-sanitizer-lexer full
  compare_fixture parser "$PARSER_FIXTURES/control-expressions.mlg" fast-sanitizer-parser full
  compare_fixture semantic \
    "$SEMANTIC_FIXTURES/closure-nested-mutable.mlg" \
    fast-sanitizer-semantic \
    full
  compare_fixture ir "$IR_FIXTURES/place-overwrite-cleanup.mlg" fast-sanitizer-ir full
fi

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
  compare_fixture parser "$fixture" "$stem" "$corpus_profile"
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
  "$CLANG_BIN" "${OPTIMIZED_FLAGS[@]}" "$accounting_source" -o "$binary-accounting"
  "$CLANG_BIN" "${SANITIZER_FLAGS[@]}" "$accounting_source" -o "$binary-san"
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

echo "self-hosting B2e2c3s $MODE gate passed: parser-corpus=$parser_corpus_count elapsed=$((SECONDS - gate_started))s"
