#!/usr/bin/env bash
set -euo pipefail

if [[ $# -ne 1 ]]; then
  echo "usage: scripts/check-allocation-runtime.sh <generated-c>" >&2
  exit 2
fi

GENERATED_C="$1"
CLANG_BIN="${CLANG:-clang}"
OUT_DIR="target/mallang/allocation-runtime"
EXPECTED_OUTPUT=$'4\n5\n3\n2\n2\n6'

mkdir -p "$OUT_DIR"
GENERATED_C_ABS="$(cd "$(dirname "$GENERATED_C")" && pwd)/$(basename "$GENERATED_C")"

cat >"$OUT_DIR/accounting.c" <<EOF
#define main mallang_example_main
#include "$GENERATED_C_ABS"
#undef main

int main(void) {
    if (mallang_live_allocation_count() != 0) {
        fprintf(stderr, "allocation accounting did not start at zero\n");
        return 2;
    }
    if (mallang_example_main() != 0) {
        fprintf(stderr, "Mallang example main failed\n");
        return 3;
    }
    if (mallang_live_allocation_count() != 0) {
        fprintf(stderr, "Mallang example leaked compiler-owned allocations\n");
        return 4;
    }

    uint64_t mlg_before = mallang_allocation_attempt_count();
    mlg_String mlg_owned = mallang_string_owned_copy((mlg_String){
        .mlg_data = "owned",
        .mlg_len = 5,
        .mlg_storage = MLG_STRING_STATIC
    });
    if (mallang_live_allocation_count() != 1 ||
        mallang_allocation_attempt_count() != mlg_before + 1) {
        fprintf(stderr, "owned string allocation was not accounted\n");
        return 5;
    }
    mlg_drop_string(&mlg_owned);
    if (mallang_live_allocation_count() != 0) {
        fprintf(stderr, "owned string deallocation was not accounted\n");
        return 6;
    }
    return 0;
}
EOF

cat >"$OUT_DIR/fail-second.c" <<EOF
#define MLG_ALLOCATION_FAIL_AFTER 1
#define main mallang_example_main
#include "$GENERATED_C_ABS"
#undef main

int main(void) {
    void *mlg_first = mallang_alloc(1, "first allocation failed");
    mallang_dealloc(mlg_first);
    (void)mallang_alloc(1, "second allocation failed");
    return 0;
}
EOF

cat >"$OUT_DIR/fail-string.c" <<EOF
#define MLG_ALLOCATION_FAIL_AFTER 0
#define main mallang_example_main
#include "$GENERATED_C_ABS"
#undef main

int main(void) {
    (void)mallang_string_owned_copy((mlg_String){
        .mlg_data = "owned",
        .mlg_len = 5,
        .mlg_storage = MLG_STRING_STATIC
    });
    return 0;
}
EOF

compile_strict() {
  local source="$1"
  local output="$2"
  local stderr_path="$3"
  if ! "$CLANG_BIN" -std=c11 -Wall -Wextra -Werror "$source" -o "$output" 2>"$stderr_path"; then
    cat "$stderr_path" >&2
    exit 1
  fi
  if [[ -s "$stderr_path" ]]; then
    cat "$stderr_path" >&2
    exit 1
  fi
}

expect_failure() {
  local binary="$1"
  local expected="$2"
  local stdout_path="$3"
  local stderr_path="$4"
  if "$binary" >"$stdout_path" 2>"$stderr_path"; then
    echo "allocation failure injection unexpectedly succeeded" >&2
    exit 1
  fi
  if [[ -s "$stdout_path" ]] || [[ "$(<"$stderr_path")" != "mallang runtime error: $expected" ]]; then
    echo "allocation failure injection output mismatch" >&2
    cat "$stdout_path" >&2
    cat "$stderr_path" >&2
    exit 1
  fi
}

compile_strict "$OUT_DIR/accounting.c" "$OUT_DIR/accounting" "$OUT_DIR/accounting-compile.stderr"
accounting_output="$("$OUT_DIR/accounting" 2>"$OUT_DIR/accounting-run.stderr")"
if [[ "$accounting_output" != "$EXPECTED_OUTPUT" ]] || [[ -s "$OUT_DIR/accounting-run.stderr" ]]; then
  echo "allocation accounting native output mismatch" >&2
  cat "$OUT_DIR/accounting-run.stderr" >&2
  exit 1
fi

if ! "$CLANG_BIN" -std=c11 -Wall -Wextra -Werror -fsanitize=address,undefined \
  -fno-omit-frame-pointer "$OUT_DIR/accounting.c" -o "$OUT_DIR/accounting-san" \
  2>"$OUT_DIR/accounting-san-compile.stderr"; then
  cat "$OUT_DIR/accounting-san-compile.stderr" >&2
  exit 1
fi
if [[ -s "$OUT_DIR/accounting-san-compile.stderr" ]]; then
  cat "$OUT_DIR/accounting-san-compile.stderr" >&2
  exit 1
fi
sanitized_output="$("$OUT_DIR/accounting-san" 2>"$OUT_DIR/accounting-san-run.stderr")"
if [[ "$sanitized_output" != "$EXPECTED_OUTPUT" ]] || [[ -s "$OUT_DIR/accounting-san-run.stderr" ]]; then
  echo "allocation accounting sanitizer output mismatch" >&2
  cat "$OUT_DIR/accounting-san-run.stderr" >&2
  exit 1
fi

compile_strict "$OUT_DIR/fail-second.c" "$OUT_DIR/fail-second" "$OUT_DIR/fail-second-compile.stderr"
expect_failure "$OUT_DIR/fail-second" "second allocation failed" \
  "$OUT_DIR/fail-second.stdout" "$OUT_DIR/fail-second.stderr"

compile_strict "$OUT_DIR/fail-string.c" "$OUT_DIR/fail-string" "$OUT_DIR/fail-string-compile.stderr"
expect_failure "$OUT_DIR/fail-string" "string allocation failed" \
  "$OUT_DIR/fail-string.stdout" "$OUT_DIR/fail-string.stderr"

if ! "$CLANG_BIN" -std=c11 -Wall -Wextra -Werror -DMLG_ALLOCATION_FAIL_AFTER=0 \
  "$GENERATED_C" -o "$OUT_DIR/fail-source" 2>"$OUT_DIR/fail-source-compile.stderr"; then
  cat "$OUT_DIR/fail-source-compile.stderr" >&2
  exit 1
fi
if [[ -s "$OUT_DIR/fail-source-compile.stderr" ]]; then
  cat "$OUT_DIR/fail-source-compile.stderr" >&2
  exit 1
fi
expect_failure "$OUT_DIR/fail-source" "slice allocation failed" \
  "$OUT_DIR/fail-source.stdout" "$OUT_DIR/fail-source.stderr"

echo "allocation accounting and failure injection harness passed"
