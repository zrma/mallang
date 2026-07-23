#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

MLG_BIN="${MLG_BIN:-target/debug/mlg}"
CLANG_BIN="${CLANG:-clang}"
FIXTURE="tests/fixtures/v11-streaming-io/for-each-line.mlg"
OUT_DIR="target/mallang/streaming-file-io-runtime"
GENERATED_C="target/mallang/for-each-line.c"
COMMON_FLAGS=(-std=c11 -Wall -Wextra -Werror -pedantic)

if [[ ! -x "$MLG_BIN" ]]; then
  echo "missing mlg binary: $MLG_BIN" >&2
  exit 1
fi

mkdir -p "$OUT_DIR"
"$MLG_BIN" check "$FIXTURE" >/dev/null
"$MLG_BIN" build "$FIXTURE" -o "$OUT_DIR/for-each-line" >/dev/null

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

compile_strict \
  "$GENERATED_C" \
  "$OUT_DIR/for-each-line-strict" \
  "$OUT_DIR/for-each-line-strict.stderr"

assert_run() {
  local label="$1"
  local input="$2"
  local needle="$3"
  local expected="$4"
  "$OUT_DIR/for-each-line-strict" "$input" "$needle" \
    >"$OUT_DIR/$label.stdout" 2>"$OUT_DIR/$label.stderr"
  if ! cmp -s "$expected" "$OUT_DIR/$label.stdout" || \
    [[ -s "$OUT_DIR/$label.stderr" ]]; then
    echo "streaming file I/O $label mismatch" >&2
    exit 1
  fi
}

printf 'INFO first\nERROR 둘\nERROR final' >"$OUT_DIR/mixed.txt"
printf '2\nERROR 둘\n3\nERROR final\n2\n' >"$OUT_DIR/mixed.expected"
assert_run mixed "$OUT_DIR/mixed.txt" ERROR "$OUT_DIR/mixed.expected"

printf 'a\nb\n' >"$OUT_DIR/terminal-newline.txt"
printf '1\na\n2\nb\n2\n' >"$OUT_DIR/terminal-newline.expected"
assert_run \
  terminal-newline \
  "$OUT_DIR/terminal-newline.txt" \
  '' \
  "$OUT_DIR/terminal-newline.expected"

: >"$OUT_DIR/empty.txt"
printf '0\n' >"$OUT_DIR/empty.expected"
assert_run empty "$OUT_DIR/empty.txt" '' "$OUT_DIR/empty.expected"

printf 'a\n\nb' >"$OUT_DIR/blank-line.txt"
printf '1\na\n2\n\n3\nb\n3\n' >"$OUT_DIR/blank-line.expected"
assert_run blank-line "$OUT_DIR/blank-line.txt" '' "$OUT_DIR/blank-line.expected"

printf 'ERROR first\r\nINFO second\r\n' >"$OUT_DIR/crlf.txt"
printf '1\nERROR first\r\n1\n' >"$OUT_DIR/crlf.expected"
assert_run crlf "$OUT_DIR/crlf.txt" ERROR "$OUT_DIR/crlf.expected"

printf 'A\0B\n' >"$OUT_DIR/embedded-nul.txt"
{
  printf '1\nA\0B\n1\n'
} >"$OUT_DIR/embedded-nul.expected"
assert_run embedded-nul "$OUT_DIR/embedded-nul.txt" B "$OUT_DIR/embedded-nul.expected"

head -c 1048576 /dev/zero | LC_ALL=C tr '\000' 'x' >"$OUT_DIR/long-line.txt"
printf '\n' >>"$OUT_DIR/long-line.txt"
printf '0\n' >"$OUT_DIR/long-line.expected"
assert_run long-line "$OUT_DIR/long-line.txt" MISSING "$OUT_DIR/long-line.expected"

printf '\300\257\n' >"$OUT_DIR/invalid-utf8.txt"
printf 'InvalidData\n' >"$OUT_DIR/invalid-utf8.expected"
assert_run \
  invalid-utf8 \
  "$OUT_DIR/invalid-utf8.txt" \
  ERROR \
  "$OUT_DIR/invalid-utf8.expected"

printf 'NotFound\n' >"$OUT_DIR/missing.expected"
assert_run \
  missing \
  "$OUT_DIR/does-not-exist.txt" \
  ERROR \
  "$OUT_DIR/missing.expected"

GENERATED_C_ABS="$(cd target/mallang && pwd)/for-each-line.c"
INPUT_ABS="$(cd "$OUT_DIR" && pwd)/mixed.txt"

cat >"$OUT_DIR/read-failure.c" <<EOF
#include <errno.h>
#include <stdio.h>
static size_t mallang_test_fread(void *mlg_ptr, size_t mlg_size, size_t mlg_count, FILE *mlg_file);
static int mallang_test_fgetc(FILE *mlg_file);
#define fread mallang_test_fread
#define fgetc mallang_test_fgetc
#define main mallang_example_main
#include "$GENERATED_C_ABS"
#undef main
#undef fgetc
#undef fread

static size_t mallang_test_fread(void *mlg_ptr, size_t mlg_size, size_t mlg_count, FILE *mlg_file) {
    (void)mlg_ptr;
    (void)mlg_size;
    (void)mlg_count;
    (void)mlg_file;
    errno = EIO;
    return 0;
}

static int mallang_test_fgetc(FILE *mlg_file) {
    (void)mlg_file;
    errno = EIO;
    return EOF;
}

int main(void) {
    (void)mallang_test_fread;
    (void)mallang_test_fgetc;
    char *mlg_argv[] = { "program", "$INPUT_ABS", "ERROR", NULL };
    return mallang_example_main(3, mlg_argv);
}
EOF

cat >"$OUT_DIR/close-failure.c" <<EOF
#include <errno.h>
#include <stdio.h>
static int mallang_test_fclose(FILE *mlg_file);
#define fclose mallang_test_fclose
#define main mallang_example_main
#include "$GENERATED_C_ABS"
#undef main
#undef fclose

static int mallang_test_fclose(FILE *mlg_file) {
    int mlg_status = fclose(mlg_file);
    if (mlg_status != 0) {
        return mlg_status;
    }
    errno = EIO;
    return EOF;
}

int main(void) {
    char *mlg_argv[] = { "program", "$INPUT_ABS", "MISSING", NULL };
    return mallang_example_main(3, mlg_argv);
}
EOF

cat >"$OUT_DIR/accounting.c" <<EOF
#define main mallang_example_main
#include "$GENERATED_C_ABS"
#undef main

int main(void) {
    char *mlg_argv[] = { "program", "$INPUT_ABS", "MISSING", NULL };
    if (mallang_live_allocation_count() != 0) {
        return 2;
    }
    int mlg_status = mallang_example_main(3, mlg_argv);
    if (mlg_status != 0 || mallang_live_allocation_count() != 0) {
        return 3;
    }
    return 0;
}
EOF

for harness in read-failure close-failure accounting; do
  compile_strict \
    "$OUT_DIR/$harness.c" \
    "$OUT_DIR/$harness" \
    "$OUT_DIR/$harness-compile.stderr"
done

printf 'Other\n' >"$OUT_DIR/read-failure.expected"
"$OUT_DIR/read-failure" \
  >"$OUT_DIR/read-failure.stdout" 2>"$OUT_DIR/read-failure.stderr"
if ! cmp -s "$OUT_DIR/read-failure.expected" "$OUT_DIR/read-failure.stdout" || \
  [[ -s "$OUT_DIR/read-failure.stderr" ]]; then
  echo "streaming read failure mismatch" >&2
  exit 1
fi

printf 'Other\n' >"$OUT_DIR/close-failure.expected"
"$OUT_DIR/close-failure" \
  >"$OUT_DIR/close-failure.stdout" 2>"$OUT_DIR/close-failure.stderr"
if ! cmp -s "$OUT_DIR/close-failure.expected" "$OUT_DIR/close-failure.stdout" || \
  [[ -s "$OUT_DIR/close-failure.stderr" ]]; then
  echo "streaming close failure mismatch" >&2
  exit 1
fi

printf '0\n' >"$OUT_DIR/accounting.expected"
"$OUT_DIR/accounting" >"$OUT_DIR/accounting.stdout" 2>"$OUT_DIR/accounting.stderr"
if ! cmp -s "$OUT_DIR/accounting.expected" "$OUT_DIR/accounting.stdout" || \
  [[ -s "$OUT_DIR/accounting.stderr" ]]; then
  echo "streaming allocation accounting mismatch" >&2
  exit 1
fi

for harness in for-each-line read-failure close-failure accounting; do
  source="$GENERATED_C"
  if [[ "$harness" != "for-each-line" ]]; then
    source="$OUT_DIR/$harness.c"
  fi
  compile_strict \
    "$source" \
    "$OUT_DIR/$harness-san" \
    "$OUT_DIR/$harness-san-compile.stderr" \
    -fsanitize=address,undefined \
    -fno-omit-frame-pointer
done

"$OUT_DIR/for-each-line-san" "$OUT_DIR/mixed.txt" ERROR \
  >"$OUT_DIR/for-each-line-san.stdout" 2>"$OUT_DIR/for-each-line-san.stderr"
if ! cmp -s "$OUT_DIR/mixed.expected" "$OUT_DIR/for-each-line-san.stdout" || \
  [[ -s "$OUT_DIR/for-each-line-san.stderr" ]]; then
  echo "streaming sanitizer success-path mismatch" >&2
  exit 1
fi

for harness in read-failure close-failure accounting; do
  "$OUT_DIR/$harness-san" \
    >"$OUT_DIR/$harness-san.stdout" 2>"$OUT_DIR/$harness-san.stderr"
  if ! cmp -s "$OUT_DIR/$harness.expected" "$OUT_DIR/$harness-san.stdout" || \
    [[ -s "$OUT_DIR/$harness-san.stderr" ]]; then
    echo "streaming sanitizer $harness mismatch" >&2
    exit 1
  fi
done

echo "streaming file I/O UTF-8, failure, allocation, and sanitizer harness passed"
