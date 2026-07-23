#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

if [[ $# -gt 0 ]]; then
  MLG_COMMAND=("$@")
else
  MLG_COMMAND=("${MLG_BIN:-target/debug/mlg}")
fi
CLANG_BIN="${CLANG:-clang}"
OUT_DIR="target/mallang/process-io-runtime"
FIXTURE_DIR="tests/fixtures/v06-process-io"
PROCESS_C="target/mallang/process-io.c"
COMMON_FLAGS=(-std=c11 -Wall -Wextra -Werror -pedantic)

if [[ ! -x "${MLG_COMMAND[0]}" ]]; then
  echo "missing mlg binary: ${MLG_COMMAND[0]}" >&2
  exit 1
fi
if [[ ! -f "$PROCESS_C" ]]; then
  echo "missing generated process I/O C: $PROCESS_C" >&2
  exit 1
fi

mkdir -p "$OUT_DIR"
unset MALLANG_P149_MISSING

build_fixture() {
  local name="$1"
  "${MLG_COMMAND[@]}" check "$FIXTURE_DIR/$name.mlg" >/dev/null
  "${MLG_COMMAND[@]}" build "$FIXTURE_DIR/$name.mlg" -o "$OUT_DIR/$name" >/dev/null
}

for fixture in \
  args \
  env \
  env-from-stdin \
  stdin \
  stderr \
  exit \
  invalid-exit \
  read-failure \
  write-failure; do
  build_fixture "$fixture"
done

compile_strict() {
  local source="$1"
  local output="$2"
  local stderr_path="$3"
  shift 3
  if ! "$CLANG_BIN" "${COMMON_FLAGS[@]}" "$@" "$source" -o "$output" 2>"$stderr_path"; then
    cat "$stderr_path" >&2
    exit 1
  fi
  if [[ -s "$stderr_path" ]]; then
    cat "$stderr_path" >&2
    exit 1
  fi
}

for fixture in \
  args \
  env \
  env-from-stdin \
  stdin \
  stderr \
  exit \
  invalid-exit \
  read-failure \
  write-failure; do
  compile_strict \
    "target/mallang/$fixture.c" \
    "$OUT_DIR/$fixture-strict" \
    "$OUT_DIR/$fixture-strict.stderr"
done

printf '3\nalpha\n한\n' >"$OUT_DIR/args.expected"
"$OUT_DIR/args" alpha 한 >"$OUT_DIR/args.stdout" 2>"$OUT_DIR/args.stderr"
if ! cmp -s "$OUT_DIR/args.expected" "$OUT_DIR/args.stdout" || \
  [[ -s "$OUT_DIR/args.stderr" ]]; then
  echo "direct process argument output mismatch" >&2
  exit 1
fi

"${MLG_COMMAND[@]}" run "$FIXTURE_DIR/args.mlg" -- alpha 한 \
  >"$OUT_DIR/args-run.stdout" 2>"$OUT_DIR/args-run.stderr"
if ! cmp -s "$OUT_DIR/args.expected" "$OUT_DIR/args-run.stdout" || \
  [[ -s "$OUT_DIR/args-run.stderr" ]]; then
  echo "mlg run process argument forwarding mismatch" >&2
  exit 1
fi

printf 'Error{kind: InvalidData, message: process argument is not valid UTF-8}\n' \
  >"$OUT_DIR/args-run-invalid-utf8.expected"
invalid_argument=$'\300\257'
"${MLG_COMMAND[@]}" run "$FIXTURE_DIR/args.mlg" -- "$invalid_argument" \
  >"$OUT_DIR/args-run-invalid-utf8.stdout" \
  2>"$OUT_DIR/args-run-invalid-utf8.stderr"
if ! cmp -s \
  "$OUT_DIR/args-run-invalid-utf8.expected" \
  "$OUT_DIR/args-run-invalid-utf8.stdout" || \
  [[ -s "$OUT_DIR/args-run-invalid-utf8.stderr" ]]; then
  echo "mlg run invalid UTF-8 argument forwarding mismatch" >&2
  exit 1
fi

if "${MLG_COMMAND[@]}" run "$FIXTURE_DIR/args.mlg" alpha \
  >"$OUT_DIR/run-without-separator.stdout" 2>"$OUT_DIR/run-without-separator.stderr"; then
  echo "mlg run accepted program arguments without --" >&2
  exit 1
fi
if [[ -s "$OUT_DIR/run-without-separator.stdout" ]] || \
  ! grep -Fq 'program arguments must follow `--`' "$OUT_DIR/run-without-separator.stderr"; then
  echo "mlg run separator diagnostic mismatch" >&2
  exit 1
fi

printf '값\nmissing\n' >"$OUT_DIR/env.expected"
MALLANG_P149_TEST=값 "$OUT_DIR/env" \
  >"$OUT_DIR/env.stdout" 2>"$OUT_DIR/env.stderr"
if ! cmp -s "$OUT_DIR/env.expected" "$OUT_DIR/env.stdout" || \
  [[ -s "$OUT_DIR/env.stderr" ]]; then
  echo "environment present/missing output mismatch" >&2
  exit 1
fi

printf '한\0input' >"$OUT_DIR/stdin.input"
cp "$OUT_DIR/stdin.input" "$OUT_DIR/stdin.expected"
"$OUT_DIR/stdin" <"$OUT_DIR/stdin.input" \
  >"$OUT_DIR/stdin.stdout" 2>"$OUT_DIR/stdin.stderr"
if ! cmp -s "$OUT_DIR/stdin.expected" "$OUT_DIR/stdin.stdout" || \
  [[ -s "$OUT_DIR/stdin.stderr" ]]; then
  echo "standard input/output byte preservation mismatch" >&2
  exit 1
fi

printf '\300\257' >"$OUT_DIR/invalid-utf8.input"
printf 'Error{kind: InvalidData, message: standard input is not valid UTF-8}\n' \
  >"$OUT_DIR/invalid-utf8.expected"
"$OUT_DIR/stdin" <"$OUT_DIR/invalid-utf8.input" \
  >"$OUT_DIR/invalid-utf8.stdout" 2>"$OUT_DIR/invalid-utf8.stderr"
if ! cmp -s "$OUT_DIR/invalid-utf8.expected" "$OUT_DIR/invalid-utf8.stdout" || \
  [[ -s "$OUT_DIR/invalid-utf8.stderr" ]]; then
  echo "invalid UTF-8 stdin result mismatch" >&2
  exit 1
fi

printf 'A\0B' >"$OUT_DIR/nul-name.input"
printf 'InvalidInput\n' >"$OUT_DIR/nul-name.expected"
"$OUT_DIR/env-from-stdin" <"$OUT_DIR/nul-name.input" \
  >"$OUT_DIR/nul-name.stdout" 2>"$OUT_DIR/nul-name.stderr"
if ! cmp -s "$OUT_DIR/nul-name.expected" "$OUT_DIR/nul-name.stdout" || \
  [[ -s "$OUT_DIR/nul-name.stderr" ]]; then
  echo "embedded NUL environment-name result mismatch" >&2
  exit 1
fi

printf 'stderr-text' >"$OUT_DIR/stderr.expected"
"$OUT_DIR/stderr" >"$OUT_DIR/stderr.stdout" 2>"$OUT_DIR/stderr.output"
if [[ -s "$OUT_DIR/stderr.stdout" ]] || \
  ! cmp -s "$OUT_DIR/stderr.expected" "$OUT_DIR/stderr.output"; then
  echo "standard error exact output mismatch" >&2
  exit 1
fi

if "$OUT_DIR/exit" >"$OUT_DIR/exit.stdout" 2>"$OUT_DIR/exit.stderr"; then
  echo "os.exit(7) unexpectedly returned success" >&2
  exit 1
else
  exit_status=$?
fi
if [[ "$exit_status" -ne 7 ]] || [[ -s "$OUT_DIR/exit.stdout" ]] || \
  [[ -s "$OUT_DIR/exit.stderr" ]]; then
  echo "os.exit status mismatch" >&2
  exit 1
fi

if "${MLG_COMMAND[@]}" run "$FIXTURE_DIR/exit.mlg" \
  >"$OUT_DIR/exit-run.stdout" 2>"$OUT_DIR/exit-run.stderr"; then
  echo "mlg run did not propagate os.exit(7)" >&2
  exit 1
else
  run_exit_status=$?
fi
if [[ "$run_exit_status" -ne 7 ]] || [[ -s "$OUT_DIR/exit-run.stdout" ]] || \
  [[ -s "$OUT_DIR/exit-run.stderr" ]]; then
  echo "mlg run os.exit status mismatch" >&2
  exit 1
fi

if "$OUT_DIR/invalid-exit" \
  >"$OUT_DIR/invalid-exit.stdout" 2>"$OUT_DIR/invalid-exit.stderr"; then
  echo "out-of-range os.exit unexpectedly succeeded" >&2
  exit 1
fi
if [[ -s "$OUT_DIR/invalid-exit.stdout" ]] || \
  [[ "$(<"$OUT_DIR/invalid-exit.stderr")" != \
    'mallang runtime error: process exit code out of range' ]]; then
  echo "out-of-range os.exit diagnostic mismatch" >&2
  exit 1
fi

ARGS_C_ABS="$(cd target/mallang && pwd)/args.c"
ENV_C_ABS="$(cd target/mallang && pwd)/env.c"
PROCESS_C_ABS="$(cd target/mallang && pwd)/process-io.c"
READ_FAILURE_C_ABS="$(cd target/mallang && pwd)/read-failure.c"
WRITE_FAILURE_C_ABS="$(cd target/mallang && pwd)/write-failure.c"

cat >"$OUT_DIR/invalid-args.c" <<EOF
#define main mallang_example_main
#include "$ARGS_C_ABS"
#undef main

int main(void) {
    char mlg_invalid[] = { (char)0xc0, (char)0xaf, '\0' };
    char *mlg_argv[] = { "program", mlg_invalid, NULL };
    return mallang_example_main(2, mlg_argv);
}
EOF

cat >"$OUT_DIR/invalid-env.c" <<EOF
#define _POSIX_C_SOURCE 200809L
#define main mallang_example_main
#include "$ENV_C_ABS"
#undef main

int main(void) {
    char mlg_invalid[] = { (char)0xc0, (char)0xaf, '\0' };
    if (setenv("MALLANG_P149_TEST", mlg_invalid, 1) != 0) {
        return 2;
    }
    if (unsetenv("MALLANG_P149_MISSING") != 0) {
        return 3;
    }
    return mallang_example_main();
}
EOF

cat >"$OUT_DIR/accounting.c" <<EOF
#define main mallang_example_main
#include "$PROCESS_C_ABS"
#undef main

int main(int argc, char **argv) {
    if (mallang_live_allocation_count() != 0) {
        return 2;
    }
    int mlg_status = mallang_example_main(argc, argv);
    if (mlg_status != 0 || mallang_live_allocation_count() != 0) {
        return 3;
    }
    return 0;
}
EOF

cat >"$OUT_DIR/read-failure.c" <<EOF
#define _POSIX_C_SOURCE 200809L
#define main mallang_example_main
#include "$READ_FAILURE_C_ABS"
#undef main
#include <unistd.h>

int main(void) {
    if (close(STDIN_FILENO) != 0) {
        return 2;
    }
    return mallang_example_main();
}
EOF

cat >"$OUT_DIR/write-failure.c" <<EOF
#define _POSIX_C_SOURCE 200809L
#define main mallang_example_main
#include "$WRITE_FAILURE_C_ABS"
#undef main
#include <unistd.h>

int main(void) {
    if (close(STDOUT_FILENO) != 0) {
        return 2;
    }
    return mallang_example_main();
}
EOF

for harness in invalid-args invalid-env accounting read-failure write-failure; do
  compile_strict \
    "$OUT_DIR/$harness.c" \
    "$OUT_DIR/$harness" \
    "$OUT_DIR/$harness-compile.stderr"
done

printf 'Error{kind: InvalidData, message: process argument is not valid UTF-8}\n' \
  >"$OUT_DIR/invalid-args.expected"
"$OUT_DIR/invalid-args" \
  >"$OUT_DIR/invalid-args.stdout" 2>"$OUT_DIR/invalid-args.stderr"
if ! cmp -s "$OUT_DIR/invalid-args.expected" "$OUT_DIR/invalid-args.stdout" || \
  [[ -s "$OUT_DIR/invalid-args.stderr" ]]; then
  echo "invalid UTF-8 argument result mismatch" >&2
  exit 1
fi

printf 'Error{kind: InvalidData, message: environment value is not valid UTF-8}\nmissing\n' \
  >"$OUT_DIR/invalid-env.expected"
"$OUT_DIR/invalid-env" \
  >"$OUT_DIR/invalid-env.stdout" 2>"$OUT_DIR/invalid-env.stderr"
if ! cmp -s "$OUT_DIR/invalid-env.expected" "$OUT_DIR/invalid-env.stdout" || \
  [[ -s "$OUT_DIR/invalid-env.stderr" ]]; then
  echo "invalid UTF-8 environment result mismatch" >&2
  exit 1
fi

if "$OUT_DIR/read-failure" \
  >"$OUT_DIR/read-failure.stdout" 2>"$OUT_DIR/read-failure.stderr"; then
  echo "closed stdin unexpectedly produced success" >&2
  exit 1
else
  read_failure_status=$?
fi
if [[ "$read_failure_status" -ne 43 ]]; then
  echo "closed stdin result status mismatch: $read_failure_status" >&2
  exit 1
fi

if "$OUT_DIR/write-failure" \
  >"$OUT_DIR/write-failure.stdout" 2>"$OUT_DIR/write-failure.stderr"; then
  echo "closed stdout unexpectedly produced success" >&2
  exit 1
else
  write_failure_status=$?
fi
if [[ "$write_failure_status" -ne 42 ]]; then
  echo "closed stdout result status mismatch: $write_failure_status" >&2
  exit 1
fi

printf 'input' >"$OUT_DIR/process.input"
printf '2\nalpha\n값\ninput' >"$OUT_DIR/process.expected"
printf 'stderr' >"$OUT_DIR/process-stderr.expected"
MALLANG_P149_TEST=값 "$OUT_DIR/accounting" alpha <"$OUT_DIR/process.input" \
  >"$OUT_DIR/accounting.stdout" 2>"$OUT_DIR/accounting.stderr"
if ! cmp -s "$OUT_DIR/process.expected" "$OUT_DIR/accounting.stdout" || \
  ! cmp -s "$OUT_DIR/process-stderr.expected" "$OUT_DIR/accounting.stderr"; then
  echo "process I/O allocation accounting output mismatch" >&2
  exit 1
fi

compile_strict \
  "$OUT_DIR/accounting.c" \
  "$OUT_DIR/accounting-san" \
  "$OUT_DIR/accounting-san-compile.stderr" \
  -fsanitize=address,undefined \
  -fno-omit-frame-pointer
MALLANG_P149_TEST=값 "$OUT_DIR/accounting-san" alpha <"$OUT_DIR/process.input" \
  >"$OUT_DIR/accounting-san.stdout" 2>"$OUT_DIR/accounting-san.stderr"
if ! cmp -s "$OUT_DIR/process.expected" "$OUT_DIR/accounting-san.stdout" || \
  ! cmp -s "$OUT_DIR/process-stderr.expected" "$OUT_DIR/accounting-san.stderr"; then
  echo "process I/O sanitizer output mismatch" >&2
  exit 1
fi

for harness in invalid-args invalid-env; do
  compile_strict \
    "$OUT_DIR/$harness.c" \
    "$OUT_DIR/$harness-san" \
    "$OUT_DIR/$harness-san-compile.stderr" \
    -fsanitize=address,undefined \
    -fno-omit-frame-pointer
  "$OUT_DIR/$harness-san" \
    >"$OUT_DIR/$harness-san.stdout" 2>"$OUT_DIR/$harness-san.stderr"
  if ! cmp -s "$OUT_DIR/$harness.expected" "$OUT_DIR/$harness-san.stdout" || \
    [[ -s "$OUT_DIR/$harness-san.stderr" ]]; then
    echo "$harness sanitizer output mismatch" >&2
    exit 1
  fi
done

first_non_failing_index=""
for ((index = 0; index < 64; index = index + 1)); do
  binary="$OUT_DIR/fail-$index"
  compile_stderr="$OUT_DIR/fail-$index-compile.stderr"
  compile_strict \
    "$PROCESS_C" \
    "$binary" \
    "$compile_stderr" \
    "-DMLG_ALLOCATION_FAIL_AFTER=$index"

  if MALLANG_P149_TEST=값 "$binary" alpha <"$OUT_DIR/process.input" \
    >"$OUT_DIR/fail-$index.stdout" 2>"$OUT_DIR/fail-$index.stderr"; then
    if ! cmp -s "$OUT_DIR/process.expected" "$OUT_DIR/fail-$index.stdout" || \
      ! cmp -s "$OUT_DIR/process-stderr.expected" "$OUT_DIR/fail-$index.stderr"; then
      echo "process I/O non-failing injection output mismatch at index $index" >&2
      exit 1
    fi
    first_non_failing_index="$index"
    break
  fi

  if ! grep -Eq '^mallang runtime error: .*allocation failed$' \
    "$OUT_DIR/fail-$index.stderr"; then
    echo "process I/O allocation failure diagnostic mismatch at index $index" >&2
    cat "$OUT_DIR/fail-$index.stderr" >&2
    exit 1
  fi
done

if [[ -z "$first_non_failing_index" ]] || [[ "$first_non_failing_index" -eq 0 ]]; then
  echo "process I/O allocation failure sweep did not reach a valid boundary" >&2
  exit 1
fi

echo "process arguments, environment, streams, exit, allocation, and sanitizer harness passed"
