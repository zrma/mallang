#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

usage() {
  cat >&2 <<'EOF'
usage: scripts/check-self-hosting-lexer.sh [--fast | --focus <area> | --compiler-pair <stage1> <stage2>] [--jobs <count>]

areas: lexer parser packages linker specialize semantic ir standard
EOF
}

MODE="full"
FOCUS=""
PAIR_MODE=false
PAIR_STAGE1=""
PAIR_STAGE2=""
JOBS="${SELF_HOSTING_JOBS:-}"
while [[ $# -gt 0 ]]; do
  case "$1" in
    --fast)
      if [[ "$MODE" != "full" || "$PAIR_MODE" == true ]]; then
        usage
        exit 2
      fi
      MODE="fast"
      shift
      ;;
    --focus)
      if [[ "$MODE" != "full" || "$PAIR_MODE" == true || $# -lt 2 ]]; then
        usage
        exit 2
      fi
      MODE="focused"
      FOCUS="$2"
      shift 2
      ;;
    --compiler-pair)
      if [[ "$MODE" != "full" || "$PAIR_MODE" == true || $# -lt 3 ]]; then
        usage
        exit 2
      fi
      PAIR_MODE=true
      PAIR_STAGE1="$2"
      PAIR_STAGE2="$3"
      shift 3
      ;;
    --jobs)
      if [[ $# -lt 2 ]]; then
        usage
        exit 2
      fi
      JOBS="$2"
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

case "$FOCUS" in
  ""|lexer|parser|packages|linker|specialize|semantic|ir|standard)
    ;;
  *)
    echo "unknown self-hosting focus area: $FOCUS" >&2
    usage
    exit 2
    ;;
esac

if [[ -z "$JOBS" ]]; then
  JOBS="$(getconf _NPROCESSORS_ONLN 2>/dev/null || echo 1)"
  if ((JOBS > 4)); then
    JOBS=4
  fi
fi
if [[ ! "$JOBS" =~ ^[1-9][0-9]*$ ]]; then
  echo "self-hosting jobs must be a positive integer: $JOBS" >&2
  exit 2
fi

CARGO=()
RUSTC=()
CLANG_BIN="${CLANG:-clang}"
if [[ "$PAIR_MODE" == false ]]; then
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

  command -v "$CLANG_BIN" >/dev/null 2>&1 || {
    echo "self-hosting lexer check failed: clang is required" >&2
    exit 1
  }
fi

WORK="target/mallang/self-hosting/b1-lexer"
STAGE0="target/debug/mlg"
STAGE1="$WORK/bootstrap-frontend"
ORACLE="$WORK/bootstrap-frontend-oracle"
if [[ "$PAIR_MODE" == true ]]; then
  if [[ ! -x "$PAIR_STAGE1" || ! -x "$PAIR_STAGE2" ]]; then
    echo "self-hosting compiler-pair check requires two executable compilers" >&2
    exit 1
  fi
  WORK="target/mallang/self-hosting/b4-fixed-point/conformance"
  STAGE1="$PAIR_STAGE2"
  ORACLE="$PAIR_STAGE1"
fi
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
STANDARD_FIXTURES="$PROJECT/fixtures/standard"
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
phase_started=$SECONDS

report_phase() {
  local phase="$1"
  echo "self-hosting timing: phase=$phase elapsed=$((SECONDS - phase_started))s"
  phase_started=$SECONDS
}

if [[ "$PAIR_MODE" == false ]]; then
"${CARGO[@]}" build --locked --quiet --lib --bin mlg
"$STAGE0" fmt --check "$PROJECT"
"$STAGE0" check "$PROJECT" >/dev/null
if [[ "$MODE" == "full" || "$MODE" == "fast" ]]; then
  "$STAGE0" test "$PROJECT" >/dev/null
else
  case "$FOCUS" in
    lexer)
      focused_tests=(
        bootstrap_compiler/frontend/lexer::NormalizesKeywordsOperatorsAndPayloads
        bootstrap_compiler/frontend/lexer::PreservesExplicitSourceIds
      )
      ;;
    parser)
      focused_tests=(
        bootstrap_compiler/frontend/parser::RecoversMultipleParserDiagnostics
        bootstrap_compiler/frontend/parser::MergesSourceAwareProgramsByDeclarationGroup
      )
      ;;
    packages)
      focused_tests=(
        bootstrap_compiler/packages::BuildsCrossProjectDependencyGraph
        bootstrap_compiler/packages::RejectsUndeclaredTransitiveProjectImport
      )
      ;;
    linker)
      focused_tests=(
        bootstrap_compiler/linker::RewritesPackageSymbolsAndPreservesLexicalShadowing
        bootstrap_compiler/linker::PreservesStandardFunctionIdentity
      )
      ;;
    specialize)
      focused_tests=(
        bootstrap_compiler/specialize::SpecializesGenericStructsFunctionsAndReceivers
        bootstrap_compiler/specialize::SpecializesGenericEnumsAndPreservesPatternOrigins
      )
      ;;
    semantic)
      focused_tests=(
        bootstrap_compiler/semantic::ChecksPrintStatementReads
        bootstrap_compiler/semantic::PropagatesNestedMutableCaptures
      )
      ;;
    ir)
      focused_tests=(
        bootstrap_compiler/ir::NormalizesMatchExpressionOuterCleanup
        bootstrap_compiler/ir::LowersPlaceAndTemporaryMethodReceivers
      )
      ;;
    standard)
      focused_tests=(
        bootstrap_compiler/standard::AugmentsCompilerOwnedStandardDeclarations
        bootstrap_compiler/standard::PreservesIntrinsicIdentityThroughTypedIr
        bootstrap_compiler/standard::RejectsUnsupportedMapKeyTypes
      )
      ;;
  esac
  for test_id in "${focused_tests[@]}"; do
    "$STAGE0" test "$PROJECT" --exact "$test_id" >/dev/null
  done
fi

"$STAGE0" build "$PROJECT" -o "$STAGE1" >/dev/null
if [[ "$MODE" != "focused" ]]; then
  cp "$GENERATED_C" "$WORK/bootstrap-frontend-first.c"
  "$STAGE0" build "$PROJECT" -o "$STAGE1" >/dev/null
  if ! cmp -s "$WORK/bootstrap-frontend-first.c" "$GENERATED_C"; then
    echo "self-hosting lexer check failed: Stage0 generated non-deterministic C" >&2
    diff -u "$WORK/bootstrap-frontend-first.c" "$GENERATED_C" >&2 || true
    exit 1
  fi
fi
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

bootstrap_build_pids=()
"$CLANG_BIN" "${OPTIMIZED_FLAGS[@]}" "$GENERATED_C" -o "$STAGE1" &
bootstrap_build_pids+=("$!")
"$CLANG_BIN" "${OPTIMIZED_FLAGS[@]}" "$WORK/accounting.c" -o "$WORK/accounting" &
bootstrap_build_pids+=("$!")
"$CLANG_BIN" "${SANITIZER_FLAGS[@]}" "$WORK/accounting.c" -o "$WORK/accounting-san" &
bootstrap_build_pids+=("$!")
"${RUSTC[@]}" \
  --edition=2021 \
  tools/bootstrap-frontend-oracle.rs \
  --extern mallang=target/debug/libmallang.rlib \
  -L dependency=target/debug/deps \
  -o "$ORACLE" &
bootstrap_build_pids+=("$!")

bootstrap_build_failed=0
for pid in "${bootstrap_build_pids[@]}"; do
  if ! wait "$pid"; then
    bootstrap_build_failed=1
  fi
done
if [[ "$bootstrap_build_failed" -ne 0 ]]; then
  echo "self-hosting bootstrap artifact build failed" >&2
  exit 1
fi

ACCOUNTING="$WORK/accounting"
SANITIZER="$WORK/accounting-san"
else
  ACCOUNTING="$STAGE1"
  SANITIZER="$STAGE1"
fi

export SELF_HOSTING_WORK="$WORK"
export SELF_HOSTING_STAGE1="$STAGE1"
export SELF_HOSTING_ORACLE="$ORACLE"
export SELF_HOSTING_ACCOUNTING="$ACCOUNTING"
export SELF_HOSTING_SANITIZER="$SANITIZER"

report_phase bootstrap

compare_fixture() {
  scripts/check-self-hosting-fixture.sh "$@"
}

queue_fixture() {
  local task_file="$1"
  shift
  printf '%s\0' "$@" >>"$task_file"
  FIXTURE_TASK_COUNT=$((FIXTURE_TASK_COUNT + 1))
}

run_fixture_tasks() {
  local task_file="$1"
  if [[ ! -s "$task_file" ]]; then
    return
  fi
  xargs -0 -n 4 -P "$JOBS" scripts/check-self-hosting-fixture.sh <"$task_file"
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
if [[ "$PAIR_MODE" == true ]]; then
  fixture_profile="stage1"
  corpus_profile="stage1"
elif [[ "$MODE" == "fast" ]]; then
  fixture_profile="strict"
  corpus_profile="stage1"
elif [[ "$MODE" == "focused" ]]; then
  fixture_profile="strict"
fi

FIXTURE_TASK_COUNT=0
FIXTURE_TASKS="$WORK/fixture-tasks.bin"
: >"$FIXTURE_TASKS"

if [[ "$MODE" == "focused" ]]; then
  case "$FOCUS" in
    lexer)
      for fixture in "$FIXTURES"/*.mlg; do
        queue_fixture "$FIXTURE_TASKS" lexer "$fixture" \
          "focused-lexer-$(basename "$fixture" .mlg)" "$fixture_profile"
      done
      queue_fixture "$FIXTURE_TASKS" lexer "$FIXTURES/all-tokens.mlg" \
        focused-sanitizer-lexer full
      ;;
    parser)
      for fixture in "$PARSER_FIXTURES"/*.mlg; do
        queue_fixture "$FIXTURE_TASKS" parser "$fixture" \
          "focused-parser-$(basename "$fixture" .mlg)" "$fixture_profile"
      done
      queue_fixture "$FIXTURE_TASKS" parser "$PARSER_FIXTURES/control-expressions.mlg" \
        focused-sanitizer-parser full
      ;;
    specialize)
      for fixture in \
        "$SEMANTIC_FIXTURES/generic-specialization.mlg" \
        "$SEMANTIC_FIXTURES/generic-enum-specialization.mlg" \
        "$SEMANTIC_FIXTURES/generic-unused-arithmetic.mlg"; do
        queue_fixture "$FIXTURE_TASKS" semantic "$fixture" \
          "focused-specialize-semantic-$(basename "$fixture" .mlg)" "$fixture_profile"
      done
      for fixture in \
        "$IR_FIXTURES/generic-specialization.mlg" \
        "$IR_FIXTURES/generic-enum-specialization.mlg"; do
        queue_fixture "$FIXTURE_TASKS" ir "$fixture" \
          "focused-specialize-ir-$(basename "$fixture" .mlg)" "$fixture_profile"
      done
      queue_fixture "$FIXTURE_TASKS" semantic \
        "$SEMANTIC_FIXTURES/generic-specialization.mlg" \
        focused-sanitizer-specialize full
      ;;
    semantic)
      for fixture in \
        "$SEMANTIC_FIXTURES/primitive-body.mlg" \
        "$SEMANTIC_FIXTURES/ownership-use-after-move.mlg" \
        "$SEMANTIC_FIXTURES/ownership-temporary-borrow-arguments.mlg" \
        "$SEMANTIC_FIXTURES/closure-nested-mutable.mlg" \
        "$SEMANTIC_FIXTURES/match-nested-user-enums.mlg" \
        "$SEMANTIC_FIXTURES/method-receiver-computed-field-index.mlg" \
        "$SEMANTIC_FIXTURES/for-post-index.mlg" \
        "$SEMANTIC_FIXTURES/test-assertions.mlg"; do
        queue_fixture "$FIXTURE_TASKS" semantic "$fixture" \
          "focused-semantic-$(basename "$fixture" .mlg)" "$fixture_profile"
      done
      queue_fixture "$FIXTURE_TASKS" semantic \
        "$SEMANTIC_FIXTURES/closure-nested-mutable.mlg" \
        focused-sanitizer-semantic full
      ;;
    ir)
      for fixture in \
        "$IR_FIXTURES/primitives.mlg" \
        "$IR_FIXTURES/branch-assignment-reactivation.mlg" \
        "$IR_FIXTURES/call-borrow-full-expression.mlg" \
        "$IR_FIXTURES/computed-place-temporary.mlg" \
        "$IR_FIXTURES/nested-cleanup-return-temp.mlg" \
        "$IR_FIXTURES/partial-field-move-cleanup.mlg" \
        "$IR_FIXTURES/pattern-copy-shadow-cleanup.mlg" \
        "$IR_FIXTURES/place-overwrite-cleanup.mlg" \
        "$IR_FIXTURES/match-nested-patterns.mlg" \
        "$IR_FIXTURES/closure-nested.mlg" \
        "$IR_FIXTURES/range-temporary-owner.mlg" \
        "$IR_FIXTURES/method-place-temporary.mlg" \
        "$IR_FIXTURES/generic-specialization.mlg"; do
        queue_fixture "$FIXTURE_TASKS" ir "$fixture" \
          "focused-ir-$(basename "$fixture" .mlg)" "$fixture_profile"
      done
      queue_fixture "$FIXTURE_TASKS" ir-test "$IR_TEST_FIXTURES/assert-statements.mlg" \
        focused-ir-test-assert-statements "$fixture_profile"
      queue_fixture "$FIXTURE_TASKS" ir "$IR_FIXTURES/place-overwrite-cleanup.mlg" \
        focused-sanitizer-ir full
      ;;
    packages|linker|standard)
      ;;
  esac
else
  for fixture in "$FIXTURES"/*.mlg; do
    queue_fixture "$FIXTURE_TASKS" lexer "$fixture" \
      "$(basename "$fixture" .mlg)" "$fixture_profile"
  done

  for fixture in "$PARSER_FIXTURES"/*.mlg; do
    queue_fixture "$FIXTURE_TASKS" parser "$fixture" \
      "parser-$(basename "$fixture" .mlg)" "$fixture_profile"
  done

  for fixture in "$SEMANTIC_FIXTURES"/*.mlg; do
    queue_fixture "$FIXTURE_TASKS" semantic "$fixture" \
      "semantic-$(basename "$fixture" .mlg)" "$fixture_profile"
  done

  for fixture in "$IR_FIXTURES"/*.mlg; do
    queue_fixture "$FIXTURE_TASKS" ir "$fixture" \
      "ir-$(basename "$fixture" .mlg)" "$fixture_profile"
  done

  for fixture in "$IR_TEST_FIXTURES"/*.mlg; do
    queue_fixture "$FIXTURE_TASKS" ir-test "$fixture" \
      "ir-test-$(basename "$fixture" .mlg)" "$fixture_profile"
  done
fi

run_fixture_tasks "$FIXTURE_TASKS"
report_phase fixture-differentials

if [[ "$MODE" != "focused" || "$FOCUS" == "parser" || "$FOCUS" == "packages" ]]; then
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
fi

if [[ "$MODE" != "focused" || "$FOCUS" == "packages" ]]; then
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
if [[ "$MODE" == "focused" ]]; then
  compare_package_layout \
    focused-sanitizer-package-layout \
    full \
    hello \
    "$PACKAGE_LAYOUT_FIXTURES/valid/src" \
    "$PACKAGE_LAYOUT_FIXTURES/valid/src/main.mlg" \
    "$PACKAGE_LAYOUT_FIXTURES/valid/src/greet/greet.mlg"
fi
fi

if [[ "$MODE" != "focused" || "$FOCUS" == "linker" ]]; then
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
if [[ "$MODE" == "focused" ]]; then
  compare_project_link \
    focused-sanitizer-linker \
    full \
    1 hello "$LINKER_FIXTURES/valid/src" 0 \
    -- \
    "$LINKER_FIXTURES/valid/src/main.mlg" \
    "$LINKER_FIXTURES/valid/src/model/model.mlg"
fi
fi

if [[ "$MODE" != "focused" || "$FOCUS" == "standard" ]]; then
for operation in augment prepare check ir; do
  compare_project_invocation \
    "standard-valid-$operation" \
    "$fixture_profile" \
    "$operation-project" \
    1 standard_valid "$STANDARD_FIXTURES/valid/src" 0 \
    "$STANDARD_FIXTURES/valid/src/main.mlg"
done
compare_project_invocation \
  standard-map-key \
  "$fixture_profile" \
  check-project \
  1 standard_map_key "$STANDARD_FIXTURES/map-key/src" 0 \
  "$STANDARD_FIXTURES/map-key/src/main.mlg"
if [[ "$MODE" == "focused" ]]; then
  compare_project_invocation \
    focused-sanitizer-standard \
    full \
    check-project \
    1 standard_valid "$STANDARD_FIXTURES/valid/src" 0 \
    "$STANDARD_FIXTURES/valid/src/main.mlg"
fi
fi

if [[ "$MODE" == "full" ]]; then
  compiler_link_sources=()
  while IFS= read -r source_path; do
    compiler_link_sources+=("$source_path")
  done < <(find bootstrap/compiler/src -type f -name '*.mlg' -print | LC_ALL=C sort)

  compiler_check_pids=()
  compare_project_link \
    linker-compiler-source \
    stage1 \
    1 bootstrap_compiler bootstrap/compiler/src 0 \
    -- \
    "${compiler_link_sources[@]}" &
  compiler_check_pids+=("$!")
  for operation in prepare check ir; do
    compare_project_invocation \
      "compiler-source-$operation" \
      stage1 \
      "$operation-project" \
      1 bootstrap_compiler bootstrap/compiler/src 0 \
      "${compiler_link_sources[@]}" &
    compiler_check_pids+=("$!")
  done

  compiler_check_failed=0
  for pid in "${compiler_check_pids[@]}"; do
    if ! wait "$pid"; then
      compiler_check_failed=1
    fi
  done
  if [[ "$compiler_check_failed" -ne 0 ]]; then
    exit 1
  fi
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

report_phase project-differentials

PARSER_CORPUS_LIST="$WORK/parser-corpus.list"
parser_corpus_count=0
if [[ "$MODE" != "focused" ]]; then
  find \
    bootstrap/compiler/src \
    bootstrap/compiler/tests \
    examples \
    tests/fixtures \
    -type f -name '*.mlg' -print | LC_ALL=C sort >"$PARSER_CORPUS_LIST"

  CORPUS_TASKS="$WORK/parser-corpus-tasks.bin"
  : >"$CORPUS_TASKS"
  while IFS= read -r fixture; do
    stem="parser-corpus-$(printf '%04d' "$parser_corpus_count")"
    queue_fixture "$CORPUS_TASKS" parser "$fixture" "$stem" "$corpus_profile"
    parser_corpus_count=$((parser_corpus_count + 1))
  done <"$PARSER_CORPUS_LIST"

  if ((parser_corpus_count < 150)); then
    echo "self-hosting parser corpus unexpectedly small: $parser_corpus_count" >&2
    exit 1
  fi
  run_fixture_tasks "$CORPUS_TASKS"
fi

report_phase parser-corpus

cleanup_regressions=()
if [[ "$PAIR_MODE" == true ]]; then
  cleanup_regressions=()
elif [[ "$MODE" != "focused" ]]; then
  cleanup_regressions=(
    append-match
    append-match-loop
    match-arm-return-temp
    pattern-copy-shadow-cleanup
    string-equality-temporaries
  )
elif [[ "$FOCUS" == "ir" ]]; then
  cleanup_regressions=(append-match pattern-copy-shadow-cleanup)
elif [[ "$FOCUS" == "semantic" ]]; then
  cleanup_regressions=(append-match)
fi

if ((${#cleanup_regressions[@]} > 0)); then
for regression in "${cleanup_regressions[@]}"; do
  fixture="tests/fixtures/self-hosting/$regression.mlg"
  if [[ "$regression" == "pattern-copy-shadow-cleanup" ]]; then
    fixture="$IR_FIXTURES/$regression.mlg"
  fi
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
fi

if [[ "$PAIR_MODE" == true ]]; then
  :
elif [[ "$MODE" != "focused" ]]; then
  if [[ "$(cat "$WORK/append-match.stdout")" != "2" ]] || \
    [[ "$(cat "$WORK/append-match-loop.stdout")" != "1" ]] || \
    [[ "$(cat "$WORK/match-arm-return-temp.stdout")" != "7" ]] || \
    [[ "$(cat "$WORK/pattern-copy-shadow-cleanup.stdout")" != $'7\n0' ]] || \
    [[ "$(cat "$WORK/string-equality-temporaries.stdout")" != $'true\ntrue' ]]; then
    echo "self-hosting lexer cleanup regression output mismatch" >&2
    exit 1
  fi
elif [[ "${#cleanup_regressions[@]}" -gt 0 && \
  "$(cat "$WORK/append-match.stdout")" != "2" ]]; then
  echo "self-hosting focused cleanup regression output mismatch" >&2
  exit 1
elif [[ "$FOCUS" == "ir" && \
  "$(cat "$WORK/pattern-copy-shadow-cleanup.stdout")" != $'7\n0' ]]; then
  echo "self-hosting focused pattern shadow cleanup output mismatch" >&2
  exit 1
fi

report_phase cleanup-regressions

if [[ "$PAIR_MODE" == true ]]; then
  echo "self-hosting compiler-pair gate passed: fixture-tasks=$FIXTURE_TASK_COUNT parser-corpus=$parser_corpus_count jobs=$JOBS elapsed=$((SECONDS - gate_started))s"
elif [[ "$MODE" == "focused" ]]; then
  echo "self-hosting B2 focused gate passed: focus=$FOCUS fixture-tasks=$FIXTURE_TASK_COUNT jobs=$JOBS elapsed=$((SECONDS - gate_started))s"
else
  echo "self-hosting B2 $MODE gate passed: parser-corpus=$parser_corpus_count jobs=$JOBS elapsed=$((SECONDS - gate_started))s"
fi
