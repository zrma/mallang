#!/usr/bin/env bash
set -euo pipefail

if [[ $# -lt 1 ]]; then
  echo "usage: scripts/check-test-workflow.sh <mlg-binary> [mlg-args...]" >&2
  exit 2
fi

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

MLG_COMMAND=("$@")
CLANG_BIN="${CLANG:-clang}"
OUT_DIR="target/mallang/test-workflow"
SUCCESS_PROJECT="examples/projects/hello"
FAILURE_PROJECT="tests/fixtures/project-test-failure"
PREFLIGHT_PROJECT="tests/fixtures/project-test-preflight"
EMPTY_PROJECT="tests/fixtures/project-test-empty"
mkdir -p "$OUT_DIR"

run_success() {
  local label="$1"
  shift
  local stdout_path="$OUT_DIR/$label.stdout"
  local stderr_path="$OUT_DIR/$label.stderr"

  if ! "$@" >"$stdout_path" 2>"$stderr_path"; then
    echo "test workflow $label failed" >&2
    cat "$stderr_path" >&2
    exit 1
  fi
  if [[ -s "$stderr_path" ]]; then
    echo "test workflow $label emitted stderr" >&2
    cat "$stderr_path" >&2
    exit 1
  fi
}

run_failure() {
  local label="$1"
  shift
  local stdout_path="$OUT_DIR/$label.stdout"
  local stderr_path="$OUT_DIR/$label.stderr"
  local status

  set +e
  "$@" >"$stdout_path" 2>"$stderr_path"
  status=$?
  set -e
  if [[ "$status" -eq 0 ]]; then
    echo "test workflow $label unexpectedly succeeded" >&2
    exit 1
  fi
}

run_success "success" "${MLG_COMMAND[@]}" test "$SUCCESS_PROJECT"
expected_success=$'test hello/greet::ReadsPrivateProductionState ... ok\ntest hello::CopyAndOwnedValues ... ok\ntest hello::GenericAndClosure ... ok\ntest hello::MapAndStandardIo ... ok\ntest hello::RecursiveAdt ... ok\ntest result: ok. 5 passed; 0 failed'
if [[ "$(cat "$OUT_DIR/success.stdout")" != "$expected_success" ]]; then
  echo "test workflow success output mismatch" >&2
  cat "$OUT_DIR/success.stdout" >&2
  exit 1
fi
if grep -Fq "hidden passing output" "$OUT_DIR/success.stdout"; then
  echo "test workflow leaked passing child output" >&2
  exit 1
fi
success_runner="$SUCCESS_PROJECT/target/mallang/tests/runner.c"
if [[ ! -f "$success_runner" ]]; then
  echo "test workflow did not generate a shared native runner" >&2
  exit 1
fi
cp "$success_runner" "$OUT_DIR/success-runner.c"

run_success \
  "exact" \
  "${MLG_COMMAND[@]}" test "$SUCCESS_PROJECT/mallang.toml" --exact "hello/greet::ReadsPrivateProductionState"
expected_exact=$'test hello/greet::ReadsPrivateProductionState ... ok\ntest result: ok. 1 passed; 0 failed'
if [[ "$(cat "$OUT_DIR/exact.stdout")" != "$expected_exact" ]]; then
  echo "test workflow exact output mismatch" >&2
  cat "$OUT_DIR/exact.stdout" >&2
  exit 1
fi

run_failure "unknown" "${MLG_COMMAND[@]}" test "$SUCCESS_PROJECT" --exact "hello::Missing"
if [[ -s "$OUT_DIR/unknown.stdout" ]] || \
  [[ "$(cat "$OUT_DIR/unknown.stderr")" != 'unknown test id `hello::Missing`' ]]; then
  echo "test workflow unknown exact diagnostic mismatch" >&2
  exit 1
fi

run_failure "standalone" "${MLG_COMMAND[@]}" test "$SUCCESS_PROJECT/tests/main_test.mlg"
if [[ -s "$OUT_DIR/standalone.stdout" ]] || \
  [[ "$(cat "$OUT_DIR/standalone.stderr")" != 'mlg test requires a project directory or `mallang.toml`' ]]; then
  echo "test workflow standalone input diagnostic mismatch" >&2
  exit 1
fi

run_failure "failure" "${MLG_COMMAND[@]}" test "$FAILURE_PROJECT"
expected_failure=$'test testfailure::FirstPasses ... ok\ntest testfailure::SecondFails ... FAILED\nvisible failing output\ntest testfailure::ThirdPasses ... ok\ntest result: FAILED. 2 passed; 1 failed'
if [[ "$(cat "$OUT_DIR/failure.stdout")" != "$expected_failure" ]] || \
  ! grep -Eq '^tests/main_test\.mlg:[0-9]+:[0-9]+: assertion failed in test `testfailure::SecondFails`$' \
    "$OUT_DIR/failure.stderr"; then
  echo "test workflow failure aggregation mismatch" >&2
  cat "$OUT_DIR/failure.stdout" >&2
  cat "$OUT_DIR/failure.stderr" >&2
  exit 1
fi
if grep -Fq "hidden passing output" "$OUT_DIR/failure.stdout" || \
  grep -Fq "application main must not run" "$OUT_DIR/failure.stdout"; then
  echo "test workflow ran application main or leaked passing output" >&2
  exit 1
fi

run_failure "preflight" "${MLG_COMMAND[@]}" test "$PREFLIGHT_PROJECT"
if [[ -s "$OUT_DIR/preflight.stdout" ]] || \
  ! grep -Fq 'assertion condition must have type `bool`' "$OUT_DIR/preflight.stderr" || \
  grep -Fq "test body must not run" "$OUT_DIR/preflight.stdout"; then
  echo "test workflow whole-suite preflight mismatch" >&2
  cat "$OUT_DIR/preflight.stdout" >&2
  cat "$OUT_DIR/preflight.stderr" >&2
  exit 1
fi

run_success "empty" "${MLG_COMMAND[@]}" test "$EMPTY_PROJECT"
if [[ "$(cat "$OUT_DIR/empty.stdout")" != 'test result: ok. 0 passed; 0 failed' ]]; then
  echo "test workflow empty suite output mismatch" >&2
  cat "$OUT_DIR/empty.stdout" >&2
  exit 1
fi

run_failure \
  "runner-invocation" \
  "$SUCCESS_PROJECT/target/mallang/tests/runner"
if [[ -s "$OUT_DIR/runner-invocation.stdout" ]] || \
  [[ "$(cat "$OUT_DIR/runner-invocation.stderr")" != \
    'mallang runtime error: invalid test runner invocation' ]]; then
  echo "test workflow runner invocation diagnostic mismatch" >&2
  exit 1
fi

c_source_abs="$(cd "$(dirname "$OUT_DIR/success-runner.c")" && pwd)/success-runner.c"
accounting_source="$OUT_DIR/runner-accounting.c"
strict_binary="$OUT_DIR/runner-strict"
sanitizer_binary="$OUT_DIR/runner-sanitized"
compile_stderr="$OUT_DIR/runner.compile.stderr"
strict_run_stderr="$OUT_DIR/runner.strict.stderr"
sanitizer_stderr="$OUT_DIR/runner.sanitizer.stderr"

cat >"$accounting_source" <<EOF
#define main mallang_test_runner_main
#include "$c_source_abs"
#undef main

int main(void) {
    if (mallang_live_allocation_count() != 0) {
        fprintf(stderr, "test allocation accounting did not start at zero\n");
        return 10;
    }
    char program[] = "mallang-test";
    char case_0[] = "0";
    char case_1[] = "1";
    char case_2[] = "2";
    char case_3[] = "3";
    char case_4[] = "4";
    char *cases[] = {case_0, case_1, case_2, case_3, case_4};
    for (size_t index = 0; index < sizeof(cases) / sizeof(cases[0]); index++) {
        char *argv[] = {program, cases[index], NULL};
        int status = mallang_test_runner_main(2, argv);
        if (status != 0) {
            fprintf(stderr, "Mallang test runner case failed\n");
            return 11;
        }
        if (mallang_live_allocation_count() != 0) {
            fprintf(stderr, "Mallang test leaked compiler-owned allocations\n");
            return 12;
        }
    }
    return 0;
}
EOF

if ! "$CLANG_BIN" -std=c11 -Wall -Wextra -Werror -pedantic \
  "$accounting_source" -o "$strict_binary" 2>"$compile_stderr"; then
  echo "test workflow shared runner strict C compile failed" >&2
  cat "$compile_stderr" >&2
  exit 1
fi
if [[ -s "$compile_stderr" ]]; then
  echo "test workflow shared runner strict C compile emitted stderr" >&2
  cat "$compile_stderr" >&2
  exit 1
fi
if ! "$strict_binary" >/dev/null 2>"$strict_run_stderr"; then
  echo "test workflow shared runner allocation accounting failed" >&2
  cat "$strict_run_stderr" >&2
  exit 1
fi
if [[ -s "$strict_run_stderr" ]]; then
  echo "test workflow shared runner strict C run emitted stderr" >&2
  cat "$strict_run_stderr" >&2
  exit 1
fi

if ! "$CLANG_BIN" -std=c11 -Wall -Wextra -Werror -pedantic \
  -fsanitize=address,undefined -fno-omit-frame-pointer \
  "$accounting_source" -o "$sanitizer_binary" 2>"$sanitizer_stderr"; then
  echo "test workflow shared runner sanitizer compile failed" >&2
  cat "$sanitizer_stderr" >&2
  exit 1
fi
if [[ -s "$sanitizer_stderr" ]]; then
  echo "test workflow shared runner sanitizer compile emitted stderr" >&2
  cat "$sanitizer_stderr" >&2
  exit 1
fi
if ! "$sanitizer_binary" >/dev/null 2>"$sanitizer_stderr"; then
  echo "test workflow shared runner sanitizer run failed" >&2
  cat "$sanitizer_stderr" >&2
  exit 1
fi
if [[ -s "$sanitizer_stderr" ]]; then
  echo "test workflow shared runner sanitizer run emitted stderr" >&2
  cat "$sanitizer_stderr" >&2
  exit 1
fi

echo "project test workflow aggregation, accounting, strict C, and sanitizer smoke passed"
