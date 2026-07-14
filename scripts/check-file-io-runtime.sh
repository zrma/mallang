#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

MLG_BIN="${MLG_BIN:-target/debug/mlg}"
CLANG_BIN="${CLANG:-clang}"
OUT_DIR="target/mallang/file-io-runtime"
FIXTURE_DIR="tests/fixtures/v06-file-io"
FILE_IO_C="target/mallang/file-io.c"
COMMON_FLAGS=(-std=c11 -Wall -Wextra -Werror -pedantic)

if [[ ! -x "$MLG_BIN" ]]; then
  echo "missing mlg binary: $MLG_BIN" >&2
  exit 1
fi
if [[ ! -f "$FILE_IO_C" ]]; then
  echo "missing generated file I/O C: $FILE_IO_C" >&2
  exit 1
fi

mkdir -p "$OUT_DIR"

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

build_fixture() {
  local name="$1"
  "$MLG_BIN" check "$FIXTURE_DIR/$name.mlg" >/dev/null
  "$MLG_BIN" build "$FIXTURE_DIR/$name.mlg" -o "$OUT_DIR/$name" >/dev/null
  compile_strict \
    "target/mallang/$name.c" \
    "$OUT_DIR/$name-strict" \
    "$OUT_DIR/$name-strict.stderr"
}

for fixture in read write read-kind write-kind nul-path; do
  build_fixture "$fixture"
done
compile_strict "$FILE_IO_C" "$OUT_DIR/file-io-strict" "$OUT_DIR/file-io-strict.stderr"

head -c 5000 /dev/zero | LC_ALL=C tr '\000' 'a' >"$OUT_DIR/input.txt"
printf '한\0tail' >>"$OUT_DIR/input.txt"
cp "$OUT_DIR/input.txt" "$OUT_DIR/input.expected"

"$OUT_DIR/file-io-strict" "$OUT_DIR/input.txt" "$OUT_DIR/output.txt" \
  >"$OUT_DIR/file-io.stdout" 2>"$OUT_DIR/file-io.stderr"
if [[ -s "$OUT_DIR/file-io.stdout" ]] || [[ -s "$OUT_DIR/file-io.stderr" ]] || \
  ! cmp -s "$OUT_DIR/input.expected" "$OUT_DIR/output.txt"; then
  echo "file read/write round-trip mismatch" >&2
  exit 1
fi

printf 'replace' >"$OUT_DIR/input.txt"
"$OUT_DIR/file-io-strict" "$OUT_DIR/input.txt" "$OUT_DIR/output.txt" \
  >"$OUT_DIR/overwrite.stdout" 2>"$OUT_DIR/overwrite.stderr"
if [[ -s "$OUT_DIR/overwrite.stdout" ]] || [[ -s "$OUT_DIR/overwrite.stderr" ]] || \
  ! cmp -s "$OUT_DIR/input.txt" "$OUT_DIR/output.txt"; then
  echo "file overwrite mismatch" >&2
  exit 1
fi
cp "$OUT_DIR/input.expected" "$OUT_DIR/input.txt"

"$OUT_DIR/read" "$OUT_DIR/input.txt" \
  >"$OUT_DIR/read.stdout" 2>"$OUT_DIR/read.stderr"
if ! cmp -s "$OUT_DIR/input.expected" "$OUT_DIR/read.stdout" || \
  [[ -s "$OUT_DIR/read.stderr" ]]; then
  echo "file read exact-byte output mismatch" >&2
  exit 1
fi

"$OUT_DIR/write" "$OUT_DIR/write-output.txt" <"$OUT_DIR/input.txt" \
  >"$OUT_DIR/write.stdout" 2>"$OUT_DIR/write.stderr"
if [[ -s "$OUT_DIR/write.stdout" ]] || [[ -s "$OUT_DIR/write.stderr" ]] || \
  ! cmp -s "$OUT_DIR/input.expected" "$OUT_DIR/write-output.txt"; then
  echo "file write exact-byte output mismatch" >&2
  exit 1
fi

missing_path="$OUT_DIR/does-not-exist.txt"
rm -f "$missing_path"
printf 'Error{kind: NotFound, message: file open failed}\n' >"$OUT_DIR/missing.expected"
"$OUT_DIR/read" "$missing_path" \
  >"$OUT_DIR/missing.stdout" 2>"$OUT_DIR/missing.stderr"
if ! cmp -s "$OUT_DIR/missing.expected" "$OUT_DIR/missing.stdout" || \
  [[ -s "$OUT_DIR/missing.stderr" ]]; then
  echo "missing file result mismatch" >&2
  exit 1
fi

missing_parent="$OUT_DIR/no-such-parent-$$"
if [[ -e "$missing_parent" ]]; then
  echo "reserved missing parent path exists: $missing_parent" >&2
  exit 1
fi
"$OUT_DIR/write" "$missing_parent/output.txt" <"$OUT_DIR/input.txt" \
  >"$OUT_DIR/missing-parent.stdout" 2>"$OUT_DIR/missing-parent.stderr"
if ! cmp -s "$OUT_DIR/missing.expected" "$OUT_DIR/missing-parent.stdout" || \
  [[ -s "$OUT_DIR/missing-parent.stderr" ]]; then
  echo "missing parent write result mismatch" >&2
  exit 1
fi

printf '\300\257' >"$OUT_DIR/invalid-utf8.txt"
printf 'Error{kind: InvalidData, message: file content is not valid UTF-8}\n' \
  >"$OUT_DIR/invalid-utf8.expected"
"$OUT_DIR/read" "$OUT_DIR/invalid-utf8.txt" \
  >"$OUT_DIR/invalid-utf8.stdout" 2>"$OUT_DIR/invalid-utf8.stderr"
if ! cmp -s "$OUT_DIR/invalid-utf8.expected" "$OUT_DIR/invalid-utf8.stdout" || \
  [[ -s "$OUT_DIR/invalid-utf8.stderr" ]]; then
  echo "invalid UTF-8 file result mismatch" >&2
  exit 1
fi

printf 'A\0B' >"$OUT_DIR/nul-path.input"
printf 'InvalidInput\n' >"$OUT_DIR/nul-path.expected"
"$OUT_DIR/nul-path" <"$OUT_DIR/nul-path.input" \
  >"$OUT_DIR/nul-path.stdout" 2>"$OUT_DIR/nul-path.stderr"
if ! cmp -s "$OUT_DIR/nul-path.expected" "$OUT_DIR/nul-path.stdout" || \
  [[ -s "$OUT_DIR/nul-path.stderr" ]]; then
  echo "embedded NUL path result mismatch" >&2
  exit 1
fi

READ_KIND_C_ABS="$(cd target/mallang && pwd)/read-kind.c"
WRITE_KIND_C_ABS="$(cd target/mallang && pwd)/write-kind.c"
FILE_IO_C_ABS="$(cd target/mallang && pwd)/file-io.c"
INPUT_ABS="$(cd "$OUT_DIR" && pwd)/input.txt"
OUTPUT_ABS="$(cd "$OUT_DIR" && pwd)/harness-output.txt"
MISSING_ABS="$(cd "$OUT_DIR" && pwd)/does-not-exist.txt"
INVALID_ABS="$(cd "$OUT_DIR" && pwd)/invalid-utf8.txt"

cat >"$OUT_DIR/permission.c" <<EOF
#include <errno.h>
#include <stdio.h>
static FILE *mallang_test_fopen(const char *mlg_path, const char *mlg_mode);
#define fopen mallang_test_fopen
#define main mallang_example_main
#include "$READ_KIND_C_ABS"
#undef main
#undef fopen

static FILE *mallang_test_fopen(const char *mlg_path, const char *mlg_mode) {
    (void)mlg_path;
    (void)mlg_mode;
    errno = EACCES;
    return NULL;
}

int main(void) {
    char *mlg_argv[] = { "program", "ignored", NULL };
    return mallang_example_main(2, mlg_argv);
}
EOF

cat >"$OUT_DIR/read-failure.c" <<EOF
#include <errno.h>
#include <stdio.h>
static size_t mallang_test_fread(void *mlg_ptr, size_t mlg_size, size_t mlg_count, FILE *mlg_file);
#define fread mallang_test_fread
#define main mallang_example_main
#include "$READ_KIND_C_ABS"
#undef main
#undef fread

static size_t mallang_test_fread(void *mlg_ptr, size_t mlg_size, size_t mlg_count, FILE *mlg_file) {
    (void)mlg_ptr;
    (void)mlg_size;
    (void)mlg_count;
    (void)mlg_file;
    errno = EIO;
    return 0;
}

int main(void) {
    char *mlg_argv[] = { "program", "$INPUT_ABS", NULL };
    return mallang_example_main(2, mlg_argv);
}
EOF

cat >"$OUT_DIR/read-close-failure.c" <<EOF
#include <errno.h>
#include <stdio.h>
static int mallang_test_fclose(FILE *mlg_file);
#define fclose mallang_test_fclose
#define main mallang_example_main
#include "$READ_KIND_C_ABS"
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
    char *mlg_argv[] = { "program", "$INPUT_ABS", NULL };
    return mallang_example_main(2, mlg_argv);
}
EOF

cat >"$OUT_DIR/write-short.c" <<EOF
#include <errno.h>
#include <stdio.h>
static size_t mallang_test_fwrite(const void *mlg_ptr, size_t mlg_size, size_t mlg_count, FILE *mlg_file);
static int mallang_test_write_calls = 0;
#define fwrite mallang_test_fwrite
#define main mallang_example_main
#include "$WRITE_KIND_C_ABS"
#undef main
#undef fwrite

static size_t mallang_test_fwrite(const void *mlg_ptr, size_t mlg_size, size_t mlg_count, FILE *mlg_file) {
    if (mlg_file == stdout || mlg_file == stderr) {
        return fwrite(mlg_ptr, mlg_size, mlg_count, mlg_file);
    }
    if (mallang_test_write_calls == 0) {
        mallang_test_write_calls = 1;
        size_t mlg_short_count = mlg_count > 1 ? mlg_count - 1 : 0;
        return fwrite(mlg_ptr, mlg_size, mlg_short_count, mlg_file);
    }
    errno = EIO;
    return 0;
}

int main(void) {
    char *mlg_argv[] = { "program", "$OUTPUT_ABS", NULL };
    return mallang_example_main(2, mlg_argv);
}
EOF

cat >"$OUT_DIR/write-close-failure.c" <<EOF
#include <errno.h>
#include <stdio.h>
static int mallang_test_fclose(FILE *mlg_file);
#define fclose mallang_test_fclose
#define main mallang_example_main
#include "$WRITE_KIND_C_ABS"
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
    char *mlg_argv[] = { "program", "$OUTPUT_ABS", NULL };
    return mallang_example_main(2, mlg_argv);
}
EOF

cat >"$OUT_DIR/accounting.c" <<EOF
#define main mallang_example_main
#include "$FILE_IO_C_ABS"
#undef main

int main(void) {
    char *mlg_argv[] = { "program", "$INPUT_ABS", "$OUTPUT_ABS", NULL };
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

for harness in permission read-failure read-close-failure write-short write-close-failure accounting; do
  compile_strict \
    "$OUT_DIR/$harness.c" \
    "$OUT_DIR/$harness" \
    "$OUT_DIR/$harness-compile.stderr"
done

expect_harness_output() {
  local harness="$1"
  local expected="$2"
  printf '%s\n' "$expected" >"$OUT_DIR/$harness.expected"
  "$OUT_DIR/$harness" >"$OUT_DIR/$harness.stdout" 2>"$OUT_DIR/$harness.stderr"
  if ! cmp -s "$OUT_DIR/$harness.expected" "$OUT_DIR/$harness.stdout" || \
    [[ -s "$OUT_DIR/$harness.stderr" ]]; then
    echo "$harness result mismatch" >&2
    exit 1
  fi
}

expect_harness_output permission PermissionDenied
expect_harness_output read-failure Other
expect_harness_output read-close-failure Other
expect_harness_output write-short Other
expect_harness_output write-close-failure Other

rm -f "$OUTPUT_ABS"
"$OUT_DIR/accounting" >"$OUT_DIR/accounting.stdout" 2>"$OUT_DIR/accounting.stderr"
if [[ -s "$OUT_DIR/accounting.stdout" ]] || [[ -s "$OUT_DIR/accounting.stderr" ]] || \
  ! cmp -s "$OUT_DIR/input.expected" "$OUTPUT_ABS"; then
  echo "file I/O allocation accounting mismatch" >&2
  exit 1
fi

for harness in permission read-failure read-close-failure write-short write-close-failure accounting; do
  compile_strict \
    "$OUT_DIR/$harness.c" \
    "$OUT_DIR/$harness-san" \
    "$OUT_DIR/$harness-san-compile.stderr" \
    -fsanitize=address,undefined \
    -fno-omit-frame-pointer
done

for harness in permission read-failure read-close-failure write-short write-close-failure; do
  "$OUT_DIR/$harness-san" \
    >"$OUT_DIR/$harness-san.stdout" 2>"$OUT_DIR/$harness-san.stderr"
  if ! cmp -s "$OUT_DIR/$harness.expected" "$OUT_DIR/$harness-san.stdout" || \
    [[ -s "$OUT_DIR/$harness-san.stderr" ]]; then
    echo "$harness sanitizer result mismatch" >&2
    exit 1
  fi
done

rm -f "$OUTPUT_ABS"
"$OUT_DIR/accounting-san" \
  >"$OUT_DIR/accounting-san.stdout" 2>"$OUT_DIR/accounting-san.stderr"
if [[ -s "$OUT_DIR/accounting-san.stdout" ]] || \
  [[ -s "$OUT_DIR/accounting-san.stderr" ]] || \
  ! cmp -s "$OUT_DIR/input.expected" "$OUTPUT_ABS"; then
  echo "file I/O sanitizer accounting mismatch" >&2
  exit 1
fi

compile_strict \
  "target/mallang/read-kind.c" \
  "$OUT_DIR/read-kind-san" \
  "$OUT_DIR/read-kind-san-compile.stderr" \
  -fsanitize=address,undefined \
  -fno-omit-frame-pointer
printf 'NotFound\n' >"$OUT_DIR/read-kind-missing.expected"
"$OUT_DIR/read-kind-san" "$MISSING_ABS" \
  >"$OUT_DIR/read-kind-missing.stdout" 2>"$OUT_DIR/read-kind-missing.stderr"
if ! cmp -s "$OUT_DIR/read-kind-missing.expected" "$OUT_DIR/read-kind-missing.stdout" || \
  [[ -s "$OUT_DIR/read-kind-missing.stderr" ]]; then
  echo "missing file sanitizer result mismatch" >&2
  exit 1
fi
printf 'InvalidData\n' >"$OUT_DIR/read-kind-invalid.expected"
"$OUT_DIR/read-kind-san" "$INVALID_ABS" \
  >"$OUT_DIR/read-kind-invalid.stdout" 2>"$OUT_DIR/read-kind-invalid.stderr"
if ! cmp -s "$OUT_DIR/read-kind-invalid.expected" "$OUT_DIR/read-kind-invalid.stdout" || \
  [[ -s "$OUT_DIR/read-kind-invalid.stderr" ]]; then
  echo "invalid UTF-8 file sanitizer result mismatch" >&2
  exit 1
fi

first_non_failing_index=""
for ((index = 0; index < 64; index = index + 1)); do
  binary="$OUT_DIR/fail-$index"
  compile_strict \
    "$FILE_IO_C" \
    "$binary" \
    "$OUT_DIR/fail-$index-compile.stderr" \
    "-DMLG_ALLOCATION_FAIL_AFTER=$index"
  rm -f "$OUTPUT_ABS"

  if "$binary" "$INPUT_ABS" "$OUTPUT_ABS" \
    >"$OUT_DIR/fail-$index.stdout" 2>"$OUT_DIR/fail-$index.stderr"; then
    if [[ -s "$OUT_DIR/fail-$index.stdout" ]] || \
      [[ -s "$OUT_DIR/fail-$index.stderr" ]] || \
      ! cmp -s "$OUT_DIR/input.expected" "$OUTPUT_ABS"; then
      echo "file I/O non-failing injection mismatch at index $index" >&2
      exit 1
    fi
    first_non_failing_index="$index"
    break
  fi

  if [[ -s "$OUT_DIR/fail-$index.stdout" ]] || \
    ! grep -Eq '^mallang runtime error: .*allocation failed$' \
      "$OUT_DIR/fail-$index.stderr"; then
    echo "file I/O allocation failure diagnostic mismatch at index $index" >&2
    exit 1
  fi
done

if [[ -z "$first_non_failing_index" ]] || [[ "$first_non_failing_index" -eq 0 ]]; then
  echo "file I/O allocation failure sweep did not reach a valid boundary" >&2
  exit 1
fi

echo "file I/O success, failure, allocation, and sanitizer harness passed"
