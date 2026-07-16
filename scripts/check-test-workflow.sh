#!/usr/bin/env bash
set -euo pipefail

if [[ $# -ne 1 ]]; then
  echo "usage: scripts/check-test-workflow.sh <mlg-binary>" >&2
  exit 2
fi

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

MLG="$1"
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

run_success "success" "$MLG" test "$SUCCESS_PROJECT"
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

run_success \
  "exact" \
  "$MLG" test "$SUCCESS_PROJECT/mallang.toml" --exact "hello/greet::ReadsPrivateProductionState"
expected_exact=$'test hello/greet::ReadsPrivateProductionState ... ok\ntest result: ok. 1 passed; 0 failed'
if [[ "$(cat "$OUT_DIR/exact.stdout")" != "$expected_exact" ]]; then
  echo "test workflow exact output mismatch" >&2
  cat "$OUT_DIR/exact.stdout" >&2
  exit 1
fi

run_failure "unknown" "$MLG" test "$SUCCESS_PROJECT" --exact "hello::Missing"
if [[ -s "$OUT_DIR/unknown.stdout" ]] || \
  [[ "$(cat "$OUT_DIR/unknown.stderr")" != 'unknown test id `hello::Missing`' ]]; then
  echo "test workflow unknown exact diagnostic mismatch" >&2
  exit 1
fi

run_failure "standalone" "$MLG" test "$SUCCESS_PROJECT/tests/main_test.mlg"
if [[ -s "$OUT_DIR/standalone.stdout" ]] || \
  [[ "$(cat "$OUT_DIR/standalone.stderr")" != 'mlg test requires a project directory or `mallang.toml`' ]]; then
  echo "test workflow standalone input diagnostic mismatch" >&2
  exit 1
fi

run_failure "failure" "$MLG" test "$FAILURE_PROJECT"
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

run_failure "preflight" "$MLG" test "$PREFLIGHT_PROJECT"
if [[ -s "$OUT_DIR/preflight.stdout" ]] || \
  ! grep -Fq 'assertion condition must have type `bool`' "$OUT_DIR/preflight.stderr" || \
  grep -Fq "test body must not run" "$OUT_DIR/preflight.stdout"; then
  echo "test workflow whole-suite preflight mismatch" >&2
  cat "$OUT_DIR/preflight.stdout" >&2
  cat "$OUT_DIR/preflight.stderr" >&2
  exit 1
fi

run_success "empty" "$MLG" test "$EMPTY_PROJECT"
if [[ "$(cat "$OUT_DIR/empty.stdout")" != 'test result: ok. 0 passed; 0 failed' ]]; then
  echo "test workflow empty suite output mismatch" >&2
  cat "$OUT_DIR/empty.stdout" >&2
  exit 1
fi

for c_source in "$SUCCESS_PROJECT"/target/mallang/tests/test-*.c; do
  stem="$(basename "$c_source" .c)"
  c_source_abs="$(cd "$(dirname "$c_source")" && pwd)/$(basename "$c_source")"
  accounting_source="$OUT_DIR/$stem-accounting.c"
  strict_binary="$OUT_DIR/$stem-strict"
  sanitizer_binary="$OUT_DIR/$stem-sanitized"
  compile_stderr="$OUT_DIR/$stem.compile.stderr"
  strict_run_stderr="$OUT_DIR/$stem.strict.stderr"
  sanitizer_stderr="$OUT_DIR/$stem.sanitizer.stderr"
  main_setup=""
  main_call="mallang_test_main()"
  if grep -Fq 'int main(int argc, char **argv)' "$c_source"; then
    main_setup=$'    char mlg_program[] = "mallang-test";\n    char *mlg_argv[] = {mlg_program, NULL};'
    main_call="mallang_test_main(1, mlg_argv)"
  fi

  cat >"$accounting_source" <<EOF
#define main mallang_test_main
#include "$c_source_abs"
#undef main

int main(void) {
    if (mallang_live_allocation_count() != 0) {
        fprintf(stderr, "test allocation accounting did not start at zero\n");
        return 10;
    }
$main_setup
    int status = $main_call;
    if (status != 0) {
        fprintf(stderr, "Mallang test main failed\n");
        return 11;
    }
    if (mallang_live_allocation_count() != 0) {
        fprintf(stderr, "Mallang test leaked compiler-owned allocations\n");
        return 12;
    }
    return 0;
}
EOF

  if ! "$CLANG_BIN" -std=c11 -Wall -Wextra -Werror -pedantic \
    "$accounting_source" -o "$strict_binary" 2>"$compile_stderr"; then
    echo "test workflow strict C compile failed for $stem" >&2
    cat "$compile_stderr" >&2
    exit 1
  fi
  if [[ -s "$compile_stderr" ]]; then
    echo "test workflow strict C compile emitted stderr for $stem" >&2
    cat "$compile_stderr" >&2
    exit 1
  fi
  if ! "$strict_binary" >/dev/null 2>"$strict_run_stderr"; then
    echo "test workflow allocation accounting failed for $stem" >&2
    cat "$strict_run_stderr" >&2
    exit 1
  fi
  if [[ -s "$strict_run_stderr" ]]; then
    echo "test workflow strict C run emitted stderr for $stem" >&2
    cat "$strict_run_stderr" >&2
    exit 1
  fi

  if ! "$CLANG_BIN" -std=c11 -Wall -Wextra -Werror -pedantic \
    -fsanitize=address,undefined -fno-omit-frame-pointer \
    "$accounting_source" -o "$sanitizer_binary" 2>"$sanitizer_stderr"; then
    echo "test workflow sanitizer compile failed for $stem" >&2
    cat "$sanitizer_stderr" >&2
    exit 1
  fi
  if [[ -s "$sanitizer_stderr" ]]; then
    echo "test workflow sanitizer compile emitted stderr for $stem" >&2
    cat "$sanitizer_stderr" >&2
    exit 1
  fi
  if ! "$sanitizer_binary" >/dev/null 2>"$sanitizer_stderr"; then
    echo "test workflow sanitizer run failed for $stem" >&2
    cat "$sanitizer_stderr" >&2
    exit 1
  fi
  if [[ -s "$sanitizer_stderr" ]]; then
    echo "test workflow sanitizer run emitted stderr for $stem" >&2
    cat "$sanitizer_stderr" >&2
    exit 1
  fi
done

echo "project test workflow aggregation, accounting, strict C, and sanitizer smoke passed"
