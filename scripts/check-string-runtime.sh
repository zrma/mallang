#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

CLANG_BIN="${CLANG:-clang}"
GENERATED_C="${1:-target/mallang/string-runtime.c}"
OUT_DIR="target/mallang/string-runtime-harness"

if [[ ! -f "$GENERATED_C" ]]; then
  echo "missing generated string runtime C: $GENERATED_C" >&2
  exit 1
fi

mkdir -p "$OUT_DIR"

cat >"$OUT_DIR/normal.c" <<EOF
#define main mallang_example_main
#include "$(cd "$(dirname "$GENERATED_C")" && pwd)/$(basename "$GENERATED_C")"
#undef main

int main(void) {
    mlg_String mlg_static = (mlg_String){
        .mlg_data = "owned",
        .mlg_len = 5,
        .mlg_storage = MLG_STRING_STATIC
    };
    mlg_String mlg_owned = mallang_string_owned_copy(mlg_static);
    if (!mallang_string_equal(mlg_static, mlg_owned)) {
        return 2;
    }
    mallang_print_string(mlg_owned);
    printf("\n");
    mlg_drop_string(&mlg_owned);
    mlg_drop_string(&mlg_static);
    return 0;
}
EOF

cat >"$OUT_DIR/malformed.c" <<EOF
#define main mallang_example_main
#include "$(cd "$(dirname "$GENERATED_C")" && pwd)/$(basename "$GENERATED_C")"
#undef main

int main(void) {
    mlg_String mlg_invalid = (mlg_String){
        .mlg_data = NULL,
        .mlg_len = 1,
        .mlg_storage = MLG_STRING_OWNED
    };
    mlg_drop_string(&mlg_invalid);
    return 0;
}
EOF

cat >"$OUT_DIR/overflow.c" <<EOF
#define main mallang_example_main
#include "$(cd "$(dirname "$GENERATED_C")" && pwd)/$(basename "$GENERATED_C")"
#undef main

int main(void) {
    mlg_String mlg_invalid = (mlg_String){
        .mlg_data = "x",
        .mlg_len = SIZE_MAX,
        .mlg_storage = MLG_STRING_STATIC
    };
    (void)mallang_string_owned_copy(mlg_invalid);
    return 0;
}
EOF

cat >"$OUT_DIR/invalid-utf8.c" <<EOF
#define main mallang_example_main
#include "$(cd "$(dirname "$GENERATED_C")" && pwd)/$(basename "$GENERATED_C")"
#undef main

int main(void) {
    const char mlg_bytes[] = { (char)0xc0, (char)0xaf };
    mlg_String mlg_invalid = (mlg_String){
        .mlg_data = mlg_bytes,
        .mlg_len = sizeof(mlg_bytes),
        .mlg_storage = MLG_STRING_STATIC
    };
    mallang_validate_string(mlg_invalid);
    return 0;
}
EOF

COMMON_FLAGS=(-std=c11 -Wall -Wextra -Werror -pedantic)
"$CLANG_BIN" "${COMMON_FLAGS[@]}" "$OUT_DIR/normal.c" -o "$OUT_DIR/normal"
if [[ "$("$OUT_DIR/normal")" != "owned" ]]; then
  echo "string runtime owned/static native output mismatch" >&2
  exit 1
fi

"$CLANG_BIN" \
  "${COMMON_FLAGS[@]}" \
  -fsanitize=address,undefined \
  -fno-omit-frame-pointer \
  "$OUT_DIR/normal.c" \
  -o "$OUT_DIR/normal-sanitized"
if [[ "$("$OUT_DIR/normal-sanitized")" != "owned" ]]; then
  echo "string runtime owned/static sanitizer output mismatch" >&2
  exit 1
fi

expect_failure() {
  local label="$1"
  local expected="$2"
  local stderr_path="$OUT_DIR/${label}.stderr"

  "$CLANG_BIN" "${COMMON_FLAGS[@]}" "$OUT_DIR/${label}.c" -o "$OUT_DIR/${label}"
  if "$OUT_DIR/${label}" >"$OUT_DIR/${label}.stdout" 2>"$stderr_path"; then
    echo "string runtime ${label} harness unexpectedly succeeded" >&2
    exit 1
  fi
  if [[ "$(cat "$stderr_path")" != "mallang runtime error: ${expected}" ]]; then
    echo "string runtime ${label} diagnostic mismatch" >&2
    exit 1
  fi
}

expect_failure malformed "invalid string data"
expect_failure overflow "string allocation size overflow"
expect_failure invalid-utf8 "invalid UTF-8 string data"

if ! rg -q 'mallang_alloc\(' "$GENERATED_C" || \
  ! rg -q '"string allocation failed"' "$GENERATED_C"; then
  echo "string runtime allocation failure guard missing" >&2
  exit 1
fi

echo "string runtime native and sanitizer harness passed"
