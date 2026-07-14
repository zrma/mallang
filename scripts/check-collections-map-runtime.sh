#!/usr/bin/env bash
set -euo pipefail

if [[ $# -ne 2 ]]; then
  echo "usage: scripts/check-collections-map-runtime.sh <example-c> <growth-c>" >&2
  exit 2
fi

EXAMPLE_C="$1"
GROWTH_C="$2"
CLANG_BIN="${CLANG:-clang}"
OUT_DIR="target/mallang/collections-map-runtime"
EXAMPLE_OUTPUT=$'inserted\n1\n1\nKim\n2\ntrue\ntrue\nKim\n3\ntrue\nfalse\nKim\n3\n0'
GROWTH_OUTPUT=$'24\n17\ntrue\n3\n23\ntrue\n11\ntrue\n20\ntrue\n7\ntrue\n7\n8\n0'
COMMON_FLAGS=(-std=c11 -Wall -Wextra -Werror -pedantic)

for source in "$EXAMPLE_C" "$GROWTH_C"; do
  if [[ ! -f "$source" ]]; then
    echo "missing generated collections Map C: $source" >&2
    exit 1
  fi
done

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

check_accounting() {
  local label="$1"
  local generated_c="$2"
  local expected_output="$3"
  local generated_abs
  generated_abs="$(cd "$(dirname "$generated_c")" && pwd)/$(basename "$generated_c")"
  local wrapper="$OUT_DIR/$label-accounting.c"

  cat >"$wrapper" <<EOF
#define main mallang_example_main
#include "$generated_abs"
#undef main

int main(void) {
    if (mallang_live_allocation_count() != 0) {
        fprintf(stderr, "collections Map accounting did not start at zero\n");
        return 2;
    }
    if (mallang_example_main() != 0) {
        fprintf(stderr, "collections Map example main failed\n");
        return 3;
    }
    if (mallang_live_allocation_count() != 0) {
        fprintf(stderr, "collections Map leaked compiler-owned allocations\n");
        return 4;
    }
    return 0;
}
EOF

  compile_strict \
    "$wrapper" \
    "$OUT_DIR/$label-accounting" \
    "$OUT_DIR/$label-accounting-compile.stderr"
  local output
  output="$("$OUT_DIR/$label-accounting" 2>"$OUT_DIR/$label-accounting-run.stderr")"
  if [[ "$output" != "$expected_output" ]] || \
    [[ -s "$OUT_DIR/$label-accounting-run.stderr" ]]; then
    echo "collections Map $label accounting output mismatch" >&2
    cat "$OUT_DIR/$label-accounting-run.stderr" >&2
    exit 1
  fi

  compile_strict \
    "$wrapper" \
    "$OUT_DIR/$label-accounting-san" \
    "$OUT_DIR/$label-accounting-san-compile.stderr" \
    -fsanitize=address,undefined \
    -fno-omit-frame-pointer
  output="$("$OUT_DIR/$label-accounting-san" 2>"$OUT_DIR/$label-accounting-san-run.stderr")"
  if [[ "$output" != "$expected_output" ]] || \
    [[ -s "$OUT_DIR/$label-accounting-san-run.stderr" ]]; then
    echo "collections Map $label sanitizer output mismatch" >&2
    cat "$OUT_DIR/$label-accounting-san-run.stderr" >&2
    exit 1
  fi
}

check_accounting "example" "$EXAMPLE_C" "$EXAMPLE_OUTPUT"
check_accounting "growth" "$GROWTH_C" "$GROWTH_OUTPUT"

new_map_thunk_count="$({
  grep -Eo 'mallang_callable_thunk_[A-Za-z0-9_]+' "$GROWTH_C" || true
} | sort -u | grep -c 'newMap')"
if [[ "$new_map_thunk_count" -ne 3 ]]; then
  echo "collections Map specialization thunk count mismatch" >&2
  exit 1
fi

first_non_failing_index=""
for ((index = 0; index < 16; index = index + 1)); do
  binary="$OUT_DIR/fail-$index"
  compile_stderr="$OUT_DIR/fail-$index-compile.stderr"
  run_stderr="$OUT_DIR/fail-$index-run.stderr"
  compile_strict \
    "$EXAMPLE_C" \
    "$binary" \
    "$compile_stderr" \
    "-DMLG_ALLOCATION_FAIL_AFTER=$index"

  if output="$("$binary" 2>"$run_stderr")"; then
    if [[ "$output" != "$EXAMPLE_OUTPUT" ]] || [[ -s "$run_stderr" ]]; then
      echo "collections Map non-failing injection output mismatch at index $index" >&2
      cat "$run_stderr" >&2
      exit 1
    fi
    first_non_failing_index="$index"
    break
  fi

  if ! grep -Eq '^mallang runtime error: map (bucket|entry) allocation failed$' "$run_stderr"; then
    echo "collections Map allocation failure diagnostic mismatch at index $index" >&2
    cat "$run_stderr" >&2
    exit 1
  fi
done

if [[ -z "$first_non_failing_index" ]] || [[ "$first_non_failing_index" -eq 0 ]]; then
  echo "collections Map allocation failure sweep did not reach a valid boundary" >&2
  exit 1
fi

for expected in \
  'map capacity overflow' \
  'map allocation size overflow' \
  'map bucket allocation failed' \
  'map entry allocation failed'; do
  if ! grep -Fq "$expected" "$EXAMPLE_C"; then
    echo "collections Map generated runtime is missing '$expected'" >&2
    exit 1
  fi
done

echo "collections Map ownership, growth, callbacks, allocation, and sanitizer harness passed"
