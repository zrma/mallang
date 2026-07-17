#!/usr/bin/env bash
set -euo pipefail

if [[ $# -ne 1 ]]; then
  echo "usage: scripts/check-self-hosting-string-cursor.sh <generated-c>" >&2
  exit 2
fi

GENERATED_C="$1"
CLANG_BIN="${CLANG:-clang}"
OUT_DIR="target/mallang/self-hosting-string-cursor-runtime"
EXPECTED_OUTPUT=$'65\n234\n128\n90\nInvalidInput\nInvalidInput\nA\n가\nZ\n0\nInvalidInput\nInvalidInput\nInvalidInput\nInvalidInput\nInvalidInput'

if [[ ! -f "$GENERATED_C" ]]; then
  echo "missing generated self-hosting string cursor C: $GENERATED_C" >&2
  exit 1
fi

mkdir -p "$OUT_DIR"
GENERATED_C_ABS="$(cd "$(dirname "$GENERATED_C")" && pwd)/$(basename "$GENERATED_C")"

cat >"$OUT_DIR/accounting.c" <<EOF
#define main mallang_example_main
#include "$GENERATED_C_ABS"
#undef main

int main(void) {
    if (mallang_live_allocation_count() != 0) {
        fprintf(stderr, "string cursor accounting did not start at zero\n");
        return 2;
    }
    if (mallang_example_main() != 0) {
        fprintf(stderr, "string cursor example main failed\n");
        return 3;
    }
    if (mallang_live_allocation_count() != 0) {
        fprintf(stderr, "string cursor example leaked compiler-owned allocations\n");
        return 4;
    }
    return 0;
}
EOF

COMMON_FLAGS=(-std=c11 -Wall -Wextra -Werror -pedantic)

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
  "$OUT_DIR/accounting.c" \
  "$OUT_DIR/accounting" \
  "$OUT_DIR/accounting-compile.stderr"
accounting_output="$("$OUT_DIR/accounting" 2>"$OUT_DIR/accounting-run.stderr")"
if [[ "$accounting_output" != "$EXPECTED_OUTPUT" ]] || \
  [[ -s "$OUT_DIR/accounting-run.stderr" ]]; then
  echo "self-hosting string cursor accounting output mismatch" >&2
  cat "$OUT_DIR/accounting-run.stderr" >&2
  exit 1
fi

compile_strict \
  "$OUT_DIR/accounting.c" \
  "$OUT_DIR/accounting-san" \
  "$OUT_DIR/accounting-san-compile.stderr" \
  -fsanitize=address,undefined \
  -fno-omit-frame-pointer
sanitized_output="$("$OUT_DIR/accounting-san" 2>"$OUT_DIR/accounting-san-run.stderr")"
if [[ "$sanitized_output" != "$EXPECTED_OUTPUT" ]] || \
  [[ -s "$OUT_DIR/accounting-san-run.stderr" ]]; then
  echo "self-hosting string cursor sanitizer mismatch" >&2
  cat "$OUT_DIR/accounting-san-run.stderr" >&2
  exit 1
fi

first_non_failing_index=""
for ((index = 0; index < 64; index = index + 1)); do
  binary="$OUT_DIR/fail-$index"
  compile_stderr="$OUT_DIR/fail-$index-compile.stderr"
  run_stderr="$OUT_DIR/fail-$index-run.stderr"
  compile_strict \
    "$GENERATED_C" \
    "$binary" \
    "$compile_stderr" \
    "-DMLG_ALLOCATION_FAIL_AFTER=$index"

  if output="$("$binary" 2>"$run_stderr")"; then
    if [[ "$output" != "$EXPECTED_OUTPUT" ]] || [[ -s "$run_stderr" ]]; then
      echo "self-hosting string cursor non-failing injection mismatch at index $index" >&2
      cat "$run_stderr" >&2
      exit 1
    fi
    first_non_failing_index="$index"
    break
  fi

  if ! grep -Eq '^mallang runtime error: .*allocation failed$' "$run_stderr"; then
    echo "self-hosting string cursor allocation diagnostic mismatch at index $index" >&2
    cat "$run_stderr" >&2
    exit 1
  fi
done

if [[ -z "$first_non_failing_index" ]] || [[ "$first_non_failing_index" -eq 0 ]]; then
  echo "self-hosting string cursor failure sweep did not reach a valid boundary" >&2
  exit 1
fi

echo "self-hosting string cursor ownership, sanitizer, and allocation gate passed"
