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
MULTI_SOURCE_FIXTURES="$PROJECT/fixtures/multi-source"
PACKAGE_LAYOUT_FIXTURES="$PROJECT/fixtures/package-layout"
LINKER_FIXTURES="$PROJECT/fixtures/linker"
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
    bootstrap_compiler/frontend/parser::MergesSourceAwareProgramsByDeclarationGroup \
    bootstrap_compiler/packages::BuildsCrossProjectDependencyGraph \
    bootstrap_compiler/packages::BuildsSourcePackageIdentity \
    bootstrap_compiler/packages::BuildsStandardPackageInventory \
    bootstrap_compiler/packages::RejectsDuplicatePackageDeclarations \
    bootstrap_compiler/packages::RejectsPackageImportCycle \
    bootstrap_compiler/packages::RejectsInvalidImportPath \
    bootstrap_compiler/packages::RejectsUndeclaredTransitiveProjectImport \
    bootstrap_compiler/packages::RejectsUnknownStandardPackage \
    bootstrap_compiler/linker::PreservesStandardFunctionIdentity \
    bootstrap_compiler/linker::RejectsMethodsOnImportedReceiverTypes \
    bootstrap_compiler/linker::RejectsPrivateImportedFunctions \
    bootstrap_compiler/linker::RejectsPrivateTypesExposedByPublicApis \
    bootstrap_compiler/linker::RewritesPackageSymbolsAndPreservesLexicalShadowing \
    bootstrap_compiler/semantic::ChecksPrintStatementReads \
    bootstrap_compiler/specialize::SpecializesGenericStructsFunctionsAndReceivers \
    bootstrap_compiler/specialize::SpecializesGenericEnumsAndPreservesPatternOrigins \
    bootstrap_compiler/specialize::RestoresSymbolicGenericBodyDiagnostics \
    bootstrap_compiler/ir::NormalizesMatchExpressionOuterCleanup; do
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

compare_source_set() {
  local stem="$1"
  local profile="$2"
  shift 2
  local -a fixtures=("$@")
  local -a actual_outputs=()
  oracle_output="$WORK/$stem.oracle"
  stage1_output="$WORK/$stem.stage1"
  strict_output="$WORK/$stem.strict"
  sanitizer_output="$WORK/$stem.sanitizer"

  "$ORACLE" parse-sources "${fixtures[@]}" >"$oracle_output"
  "$STAGE1" parse-sources "${fixtures[@]}" >"$stage1_output"
  actual_outputs+=("$stage1_output")
  if [[ "$profile" != "stage1" ]]; then
    "$WORK/accounting" parse-sources "${fixtures[@]}" \
      >"$strict_output" 2>"$WORK/$stem.strict.stderr"
    actual_outputs+=("$strict_output")
  fi
  if [[ "$profile" == "full" ]]; then
    "$WORK/accounting-san" parse-sources "${fixtures[@]}" \
      >"$sanitizer_output" 2>"$WORK/$stem.sanitizer.stderr"
    actual_outputs+=("$sanitizer_output")
  fi

  for actual in "${actual_outputs[@]}"; do
    if ! cmp -s "$oracle_output" "$actual"; then
      echo "self-hosting multi-source parser differential mismatch: $stem" >&2
      diff -u "$oracle_output" "$actual" >&2 || true
      exit 1
    fi
  done
  if [[ "$profile" != "stage1" && -s "$WORK/$stem.strict.stderr" ]]; then
    echo "self-hosting multi-source parser runtime emitted stderr: $stem" >&2
    cat "$WORK/$stem.strict.stderr" >&2
    exit 1
  fi
  if [[ "$profile" == "full" && -s "$WORK/$stem.sanitizer.stderr" ]]; then
    echo "self-hosting multi-source parser runtime emitted stderr: $stem" >&2
    cat "$WORK/$stem.sanitizer.stderr" >&2
    exit 1
  fi
}

compare_project_invocation() {
  local stem="$1"
  local profile="$2"
  shift 2
  local -a invocation=("$@")
  local -a actual_outputs=()
  oracle_output="$WORK/$stem.oracle"
  stage1_output="$WORK/$stem.stage1"
  strict_output="$WORK/$stem.strict"
  sanitizer_output="$WORK/$stem.sanitizer"

  "$ORACLE" "${invocation[@]}" >"$oracle_output"
  "$STAGE1" "${invocation[@]}" >"$stage1_output"
  actual_outputs+=("$stage1_output")
  if [[ "$profile" != "stage1" ]]; then
    "$WORK/accounting" "${invocation[@]}" \
      >"$strict_output" 2>"$WORK/$stem.strict.stderr"
    actual_outputs+=("$strict_output")
  fi
  if [[ "$profile" == "full" ]]; then
    "$WORK/accounting-san" "${invocation[@]}" \
      >"$sanitizer_output" 2>"$WORK/$stem.sanitizer.stderr"
    actual_outputs+=("$sanitizer_output")
  fi

  for actual in "${actual_outputs[@]}"; do
    if ! cmp -s "$oracle_output" "$actual"; then
      echo "self-hosting project differential mismatch: $stem" >&2
      diff -u "$oracle_output" "$actual" >&2 || true
      exit 1
    fi
  done
  if [[ "$profile" != "stage1" && -s "$WORK/$stem.strict.stderr" ]]; then
    echo "self-hosting package layout runtime emitted stderr: $stem" >&2
    cat "$WORK/$stem.strict.stderr" >&2
    exit 1
  fi
  if [[ "$profile" == "full" && -s "$WORK/$stem.sanitizer.stderr" ]]; then
    echo "self-hosting package layout runtime emitted stderr: $stem" >&2
    cat "$WORK/$stem.sanitizer.stderr" >&2
    exit 1
  fi
}

compare_package_layout() {
  local stem="$1"
  local profile="$2"
  local project_name="$3"
  local source_root="$4"
  shift 4
  compare_project_invocation \
    "$stem" \
    "$profile" \
    package-layout \
    "$project_name" \
    "$source_root" \
    "$@"
}

compare_project_package_layout() {
  local stem="$1"
  local profile="$2"
  shift 2
  local -a project_args=()
  local -a fixtures=()
  local in_fixtures=false
  local argument
  for argument in "$@"; do
    if [[ "$argument" == "--" ]]; then
      in_fixtures=true
    elif [[ "$in_fixtures" == "true" ]]; then
      fixtures+=("$argument")
    else
      project_args+=("$argument")
    fi
  done
  if [[ ${#project_args[@]} -eq 0 || ${#fixtures[@]} -eq 0 ]]; then
    echo "self-hosting project package layout requires graph and source arguments" >&2
    exit 2
  fi

  compare_project_invocation \
    "$stem" \
    "$profile" \
    package-layout-project \
    "${project_args[@]}" \
    "${fixtures[@]}"
}

compare_project_link() {
  local stem="$1"
  local profile="$2"
  shift 2
  local -a project_args=()
  local -a fixtures=()
  local in_fixtures=false
  local argument
  for argument in "$@"; do
    if [[ "$argument" == "--" ]]; then
      in_fixtures=true
    elif [[ "$in_fixtures" == "true" ]]; then
      fixtures+=("$argument")
    else
      project_args+=("$argument")
    fi
  done
  if [[ ${#project_args[@]} -eq 0 || ${#fixtures[@]} -eq 0 ]]; then
    echo "self-hosting project link requires graph and source arguments" >&2
    exit 2
  fi

  compare_project_invocation \
    "$stem" \
    "$profile" \
    link-project \
    "${project_args[@]}" \
    "${fixtures[@]}"
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

compare_source_set \
  multi-source-valid \
  "$fixture_profile" \
  "$MULTI_SOURCE_FIXTURES/valid/main.mlg" \
  "$MULTI_SOURCE_FIXTURES/valid/helper.mlg"
compare_source_set \
  multi-source-errors \
  "$fixture_profile" \
  "$MULTI_SOURCE_FIXTURES/errors/main.mlg" \
  "$MULTI_SOURCE_FIXTURES/errors/broken.mlg"

compare_package_layout \
  package-layout-valid \
  "$fixture_profile" \
  hello \
  "$PACKAGE_LAYOUT_FIXTURES/valid/src" \
  "$PACKAGE_LAYOUT_FIXTURES/valid/src/main.mlg" \
  "$PACKAGE_LAYOUT_FIXTURES/valid/src/greet/greet.mlg"
compare_package_layout \
  package-layout-missing \
  "$fixture_profile" \
  hello \
  "$PACKAGE_LAYOUT_FIXTURES/missing/src" \
  "$PACKAGE_LAYOUT_FIXTURES/missing/src/main.mlg"
compare_package_layout \
  package-layout-mismatch \
  "$fixture_profile" \
  hello \
  "$PACKAGE_LAYOUT_FIXTURES/mismatch/src" \
  "$PACKAGE_LAYOUT_FIXTURES/mismatch/src/main.mlg"
compare_package_layout \
  package-layout-invalid-import \
  "$fixture_profile" \
  hello \
  "$PACKAGE_LAYOUT_FIXTURES/invalid-import/src" \
  "$PACKAGE_LAYOUT_FIXTURES/invalid-import/src/main.mlg"
compare_package_layout \
  package-layout-unresolved \
  "$fixture_profile" \
  hello \
  "$PACKAGE_LAYOUT_FIXTURES/unresolved/src" \
  "$PACKAGE_LAYOUT_FIXTURES/unresolved/src/main.mlg"
compare_package_layout \
  package-layout-duplicate-import \
  "$fixture_profile" \
  hello \
  "$PACKAGE_LAYOUT_FIXTURES/duplicate-import/src" \
  "$PACKAGE_LAYOUT_FIXTURES/duplicate-import/src/main.mlg" \
  "$PACKAGE_LAYOUT_FIXTURES/duplicate-import/src/greet/greet.mlg"
compare_package_layout \
  package-layout-duplicate-qualifier \
  "$fixture_profile" \
  hello \
  "$PACKAGE_LAYOUT_FIXTURES/duplicate-qualifier/src" \
  "$PACKAGE_LAYOUT_FIXTURES/duplicate-qualifier/src/main.mlg" \
  "$PACKAGE_LAYOUT_FIXTURES/duplicate-qualifier/src/first/util/util.mlg" \
  "$PACKAGE_LAYOUT_FIXTURES/duplicate-qualifier/src/second/util/util.mlg"
compare_package_layout \
  package-layout-cycle \
  "$fixture_profile" \
  hello \
  "$PACKAGE_LAYOUT_FIXTURES/cycle/src" \
  "$PACKAGE_LAYOUT_FIXTURES/cycle/src/main.mlg" \
  "$PACKAGE_LAYOUT_FIXTURES/cycle/src/a/a.mlg" \
  "$PACKAGE_LAYOUT_FIXTURES/cycle/src/b/b.mlg"
compare_package_layout \
  package-layout-duplicate-declaration \
  "$fixture_profile" \
  hello \
  "$PACKAGE_LAYOUT_FIXTURES/duplicate-declaration/src" \
  "$PACKAGE_LAYOUT_FIXTURES/duplicate-declaration/src/main.mlg" \
  "$PACKAGE_LAYOUT_FIXTURES/duplicate-declaration/src/a/first.mlg" \
  "$PACKAGE_LAYOUT_FIXTURES/duplicate-declaration/src/a/second.mlg"
compare_package_layout \
  package-layout-duplicate-method \
  "$fixture_profile" \
  hello \
  "$PACKAGE_LAYOUT_FIXTURES/duplicate-method/src" \
  "$PACKAGE_LAYOUT_FIXTURES/duplicate-method/src/main.mlg" \
  "$PACKAGE_LAYOUT_FIXTURES/duplicate-method/src/a/first.mlg" \
  "$PACKAGE_LAYOUT_FIXTURES/duplicate-method/src/a/second.mlg"
compare_package_layout \
  package-layout-standard \
  "$fixture_profile" \
  hello \
  "$PACKAGE_LAYOUT_FIXTURES/standard/src" \
  "$PACKAGE_LAYOUT_FIXTURES/standard/src/main.mlg"
compare_package_layout \
  package-layout-unknown-standard \
  "$fixture_profile" \
  hello \
  "$PACKAGE_LAYOUT_FIXTURES/unknown-standard/src" \
  "$PACKAGE_LAYOUT_FIXTURES/unknown-standard/src/main.mlg"
compare_project_package_layout \
  package-layout-cross-project \
  "$fixture_profile" \
  3 \
  app "$PACKAGE_LAYOUT_FIXTURES/cross-project/src" 1 text \
  text "$PACKAGE_LAYOUT_FIXTURES/cross-project/deps/text/src" 1 shared \
  shared "$PACKAGE_LAYOUT_FIXTURES/cross-project/deps/shared/src" 0 \
  -- \
  "$PACKAGE_LAYOUT_FIXTURES/cross-project/deps/shared/src/library.mlg" \
  "$PACKAGE_LAYOUT_FIXTURES/cross-project/deps/text/src/library.mlg" \
  "$PACKAGE_LAYOUT_FIXTURES/cross-project/src/main.mlg"
compare_project_package_layout \
  package-layout-undeclared-transitive \
  "$fixture_profile" \
  3 \
  app "$PACKAGE_LAYOUT_FIXTURES/undeclared-transitive/src" 1 text \
  text "$PACKAGE_LAYOUT_FIXTURES/undeclared-transitive/deps/text/src" 1 shared \
  shared "$PACKAGE_LAYOUT_FIXTURES/undeclared-transitive/deps/shared/src" 0 \
  -- \
  "$PACKAGE_LAYOUT_FIXTURES/undeclared-transitive/deps/shared/src/library.mlg" \
  "$PACKAGE_LAYOUT_FIXTURES/undeclared-transitive/deps/text/src/library.mlg" \
  "$PACKAGE_LAYOUT_FIXTURES/undeclared-transitive/src/main.mlg"

compare_project_link \
  linker-valid \
  "$fixture_profile" \
  1 hello "$LINKER_FIXTURES/valid/src" 0 \
  -- \
  "$LINKER_FIXTURES/valid/src/main.mlg" \
  "$LINKER_FIXTURES/valid/src/model/model.mlg"
compare_project_link \
  linker-cross-project \
  "$fixture_profile" \
  3 \
  app "$PACKAGE_LAYOUT_FIXTURES/cross-project/src" 1 text \
  text "$PACKAGE_LAYOUT_FIXTURES/cross-project/deps/text/src" 1 shared \
  shared "$PACKAGE_LAYOUT_FIXTURES/cross-project/deps/shared/src" 0 \
  -- \
  "$PACKAGE_LAYOUT_FIXTURES/cross-project/deps/shared/src/library.mlg" \
  "$PACKAGE_LAYOUT_FIXTURES/cross-project/deps/text/src/library.mlg" \
  "$PACKAGE_LAYOUT_FIXTURES/cross-project/src/main.mlg"
compare_project_link \
  linker-private-function \
  "$fixture_profile" \
  1 hello "$LINKER_FIXTURES/private-function/src" 0 \
  -- \
  "$LINKER_FIXTURES/private-function/src/main.mlg" \
  "$LINKER_FIXTURES/private-function/src/greet/greet.mlg"
compare_project_link \
  linker-private-type-api \
  "$fixture_profile" \
  1 hello "$LINKER_FIXTURES/private-type-api/src" 0 \
  -- \
  "$LINKER_FIXTURES/private-type-api/src/main.mlg" \
  "$LINKER_FIXTURES/private-type-api/src/model/model.mlg"
compare_project_link \
  linker-foreign-receiver \
  "$fixture_profile" \
  1 hello "$LINKER_FIXTURES/foreign-receiver/src" 0 \
  -- \
  "$LINKER_FIXTURES/foreign-receiver/src/main.mlg" \
  "$LINKER_FIXTURES/foreign-receiver/src/greet/greet.mlg"
compare_project_link \
  linker-wrong-kind \
  "$fixture_profile" \
  1 hello "$LINKER_FIXTURES/wrong-kind/src" 0 \
  -- \
  "$LINKER_FIXTURES/wrong-kind/src/main.mlg" \
  "$LINKER_FIXTURES/wrong-kind/src/model/model.mlg"

if [[ "$MODE" == "full" ]]; then
  compiler_link_sources=()
  while IFS= read -r source_path; do
    compiler_link_sources+=("$source_path")
  done < <(find bootstrap/compiler/src -type f -name '*.mlg' -print | LC_ALL=C sort)
  compare_project_link \
    linker-compiler-source \
    stage1 \
    1 bootstrap_compiler bootstrap/compiler/src 0 \
    -- \
    "${compiler_link_sources[@]}"
fi

if [[ "$MODE" == "fast" ]]; then
  compare_fixture lexer "$FIXTURES/all-tokens.mlg" fast-sanitizer-lexer full
  compare_fixture parser "$PARSER_FIXTURES/control-expressions.mlg" fast-sanitizer-parser full
  compare_fixture semantic \
    "$SEMANTIC_FIXTURES/closure-nested-mutable.mlg" \
    fast-sanitizer-semantic \
    full
  compare_fixture ir "$IR_FIXTURES/place-overwrite-cleanup.mlg" fast-sanitizer-ir full
  compare_project_link \
    fast-sanitizer-linker \
    full \
    1 hello "$LINKER_FIXTURES/valid/src" 0 \
    -- \
    "$LINKER_FIXTURES/valid/src/main.mlg" \
    "$LINKER_FIXTURES/valid/src/model/model.mlg"
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

echo "self-hosting B2e4b3b $MODE gate passed: parser-corpus=$parser_corpus_count elapsed=$((SECONDS - gate_started))s"
