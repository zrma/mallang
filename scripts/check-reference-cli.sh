#!/usr/bin/env bash
set -euo pipefail

if [[ $# -ne 3 ]]; then
  echo "usage: scripts/check-reference-cli.sh <generated-c> <binary> <input>" >&2
  exit 2
fi

GENERATED_C="$1"
BINARY="$2"
INPUT="$3"
CLANG_BIN="${CLANG:-clang}"
OUT_DIR="target/mallang/reference-cli"
EXPECTED_OUTPUT=$'bytes=12\nscalars=10\nlines=4\ndistinct_line_lengths=3'
COMMON_FLAGS=(-std=c11 -Wall -Wextra -Werror -pedantic)

for path in "$GENERATED_C" "$BINARY" "$INPUT"; do
  if [[ ! -f "$path" ]]; then
    echo "missing reference CLI input: $path" >&2
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

check_success() {
  local label="$1"
  local command="$2"
  local input="$3"
  local stdout_path="$OUT_DIR/$label.stdout"
  local stderr_path="$OUT_DIR/$label.stderr"

  "$command" "$input" >"$stdout_path" 2>"$stderr_path"
  if [[ "$(cat "$stdout_path")" != "$EXPECTED_OUTPUT" ]] || [[ -s "$stderr_path" ]]; then
    echo "reference CLI $label output mismatch" >&2
    cat "$stderr_path" >&2
    exit 1
  fi
}

check_failure() {
  local label="$1"
  local expected_status="$2"
  local expected_stderr="$3"
  shift 3
  local stdout_path="$OUT_DIR/$label.stdout"
  local stderr_path="$OUT_DIR/$label.stderr"
  local status

  set +e
  "$@" >"$stdout_path" 2>"$stderr_path"
  status=$?
  set -e
  if [[ "$status" -ne "$expected_status" ]] || [[ -s "$stdout_path" ]] || \
    [[ "$(cat "$stderr_path")" != "$expected_stderr" ]]; then
    echo "reference CLI $label failure mismatch" >&2
    echo "status: $status" >&2
    cat "$stderr_path" >&2
    exit 1
  fi
}

check_success "built" "$BINARY" "$INPUT"

output_file="$OUT_DIR/summary.txt"
file_stdout="$OUT_DIR/file.stdout"
file_stderr="$OUT_DIR/file.stderr"
"$BINARY" "$INPUT" "$output_file" >"$file_stdout" 2>"$file_stderr"
if [[ -s "$file_stdout" ]] || [[ -s "$file_stderr" ]] || \
  [[ "$(cat "$output_file")" != "$EXPECTED_OUTPUT" ]]; then
  echo "reference CLI output-file mode mismatch" >&2
  cat "$file_stderr" >&2
  exit 1
fi

check_failure \
  "usage" \
  2 \
  "usage: textstats <input> [output]" \
  "$BINARY"
check_failure \
  "missing" \
  1 \
  "textstats: read failed: NotFound" \
  "$BINARY" "$OUT_DIR/missing.txt"

invalid_utf8="$OUT_DIR/invalid-utf8.txt"
printf '\377' >"$invalid_utf8"
check_failure \
  "invalid-utf8" \
  1 \
  "textstats: read failed: InvalidData" \
  "$BINARY" "$invalid_utf8"

write_directory="$OUT_DIR/write-directory"
mkdir -p "$write_directory"
check_failure \
  "write" \
  1 \
  "textstats: write failed: Other" \
  "$BINARY" "$INPUT" "$write_directory"

compile_strict \
  "$GENERATED_C" \
  "$OUT_DIR/strict" \
  "$OUT_DIR/strict-compile.stderr"
check_success "strict" "$OUT_DIR/strict" "$INPUT"

compile_strict \
  "$GENERATED_C" \
  "$OUT_DIR/sanitized" \
  "$OUT_DIR/sanitized-compile.stderr" \
  -fsanitize=address,undefined \
  -fno-omit-frame-pointer
check_success "sanitized" "$OUT_DIR/sanitized" "$INPUT"

generated_abs="$(cd "$(dirname "$GENERATED_C")" && pwd)/$(basename "$GENERATED_C")"
cat >"$OUT_DIR/accounting.c" <<EOF
#define main mallang_example_main
#include "$generated_abs"
#undef main

int main(int argc, char **argv) {
    if (mallang_live_allocation_count() != 0) {
        fprintf(stderr, "reference CLI accounting did not start at zero\n");
        return 10;
    }
    int status = mallang_example_main(argc, argv);
    if (status != 0 || mallang_live_allocation_count() != 0) {
        fprintf(stderr, "reference CLI leaked compiler-owned allocations\n");
        return 11;
    }
    return 0;
}
EOF
compile_strict \
  "$OUT_DIR/accounting.c" \
  "$OUT_DIR/accounting" \
  "$OUT_DIR/accounting-compile.stderr"
check_success "accounting" "$OUT_DIR/accounting" "$INPUT"

echo "reference CLI success, error flow, accounting, strict C, and sanitizer harness passed"
