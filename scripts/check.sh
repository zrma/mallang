#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

if command -v cargo >/dev/null 2>&1; then
  CARGO=(cargo)
elif command -v rustup >/dev/null 2>&1; then
  TOOLCHAIN_BIN="$HOME/.rustup/toolchains/stable-aarch64-apple-darwin/bin"
  if [[ -x "$TOOLCHAIN_BIN/cargo" ]]; then
    export PATH="$TOOLCHAIN_BIN:$PATH"
    CARGO=("$TOOLCHAIN_BIN/cargo")
  else
    CARGO=(rustup run stable cargo)
  fi
else
  TOOLCHAIN_BIN="$HOME/.rustup/toolchains/stable-aarch64-apple-darwin/bin"
  if [[ ! -x "$TOOLCHAIN_BIN/cargo" ]]; then
    echo "cargo not found and fallback toolchain missing: $TOOLCHAIN_BIN/cargo" >&2
    exit 1
  fi
  export PATH="$TOOLCHAIN_BIN:$PATH"
  CARGO=("$TOOLCHAIN_BIN/cargo")
fi

"${CARGO[@]}" fmt --all --check
"${CARGO[@]}" test --workspace
"${CARGO[@]}" clippy --workspace --all-targets -- -D warnings
"${CARGO[@]}" run --bin mlg -- examples/hello.mlg >/dev/null
"${CARGO[@]}" run --bin mlg -- parse examples/first.mlg >/dev/null
"${CARGO[@]}" run --bin mlg -- check examples/first.mlg >/dev/null
"${CARGO[@]}" run --bin mlg -- build examples/first.mlg -o target/mallang/first >/dev/null
first_output="$(target/mallang/first)"
if [[ "$first_output" != "30" ]]; then
  echo "first native build smoke failed: expected 30, got '$first_output'" >&2
  exit 1
fi
"${CARGO[@]}" run --bin mlg -- check examples/if.mlg >/dev/null
"${CARGO[@]}" run --bin mlg -- build examples/if.mlg -o target/mallang/if >/dev/null
if_output="$(target/mallang/if)"
if [[ "$if_output" != "pass" ]]; then
  echo "if native build smoke failed: expected pass, got '$if_output'" >&2
  exit 1
fi
"${CARGO[@]}" run --bin mlg -- check examples/if-statement.mlg >/dev/null
"${CARGO[@]}" run --bin mlg -- build examples/if-statement.mlg -o target/mallang/if-statement >/dev/null
if_statement_output="$(target/mallang/if-statement)"
if [[ "$if_statement_output" != "then" ]]; then
  echo "if statement native build smoke failed: expected then, got '$if_statement_output'" >&2
  exit 1
fi
"${CARGO[@]}" run --bin mlg -- check examples/adt.mlg >/dev/null
"${CARGO[@]}" run --bin mlg -- ir examples/adt.mlg >/dev/null
"${CARGO[@]}" run --bin mlg -- build examples/adt.mlg -o target/mallang/adt >/dev/null
adt_output="$(target/mallang/adt)"
if [[ "$adt_output" != $'0\n0' ]]; then
  echo "adt native build smoke failed: expected two zero lines, got '$adt_output'" >&2
  exit 1
fi
"${CARGO[@]}" run --bin mlg -- check examples/match-temp.mlg >/dev/null
"${CARGO[@]}" run --bin mlg -- build examples/match-temp.mlg -o target/mallang/match-temp >/dev/null
match_temp_output="$(target/mallang/match-temp)"
if [[ "$match_temp_output" != "0" ]]; then
  echo "match temp native build smoke failed: expected 0, got '$match_temp_output'" >&2
  exit 1
fi
"${CARGO[@]}" run --bin mlg -- check examples/structs.mlg >/dev/null
"${CARGO[@]}" run --bin mlg -- build examples/structs.mlg -o target/mallang/structs >/dev/null
structs_output="$(target/mallang/structs)"
if [[ "$structs_output" != $'kim\n30' ]]; then
  echo "structs native build smoke failed: expected kim and 30, got '$structs_output'" >&2
  exit 1
fi
"${CARGO[@]}" run --bin mlg -- check examples/methods.mlg >/dev/null
"${CARGO[@]}" run --bin mlg -- build examples/methods.mlg -o target/mallang/methods >/dev/null
methods_output="$(target/mallang/methods)"
if [[ "$methods_output" != $'kim\n30' ]]; then
  echo "methods native build smoke failed: expected kim and 30, got '$methods_output'" >&2
  exit 1
fi
"${CARGO[@]}" run --bin mlg -- check examples/mut-receiver.mlg >/dev/null
"${CARGO[@]}" run --bin mlg -- build examples/mut-receiver.mlg -o target/mallang/mut-receiver >/dev/null
mut_receiver_output="$(target/mallang/mut-receiver)"
if [[ "$mut_receiver_output" != "3" ]]; then
  echo "mut receiver native build smoke failed: expected 3, got '$mut_receiver_output'" >&2
  exit 1
fi
"${CARGO[@]}" run --bin mlg -- check examples/field-assignment.mlg >/dev/null
"${CARGO[@]}" run --bin mlg -- build examples/field-assignment.mlg -o target/mallang/field-assignment >/dev/null
field_assignment_output="$(target/mallang/field-assignment)"
if [[ "$field_assignment_output" != $'kim\n31' ]]; then
  echo "field assignment native build smoke failed: expected kim and 31, got '$field_assignment_output'" >&2
  exit 1
fi
"${CARGO[@]}" run --bin mlg -- check examples/field-borrow.mlg >/dev/null
"${CARGO[@]}" run --bin mlg -- build examples/field-borrow.mlg -o target/mallang/field-borrow >/dev/null
field_borrow_output="$(target/mallang/field-borrow)"
if [[ "$field_borrow_output" != $'kim\n30' ]]; then
  echo "field borrow native build smoke failed: expected kim and 30, got '$field_borrow_output'" >&2
  exit 1
fi
"${CARGO[@]}" run --bin mlg -- check examples/mut-parameter-abi.mlg >/dev/null
"${CARGO[@]}" run --bin mlg -- build examples/mut-parameter-abi.mlg -o target/mallang/mut-parameter-abi >/dev/null
mut_parameter_abi_output="$(target/mallang/mut-parameter-abi)"
if [[ "$mut_parameter_abi_output" != $'lee\n2' ]]; then
  echo "mut parameter ABI native build smoke failed: expected lee and 2, got '$mut_parameter_abi_output'" >&2
  exit 1
fi
"${CARGO[@]}" run --bin mlg -- check examples/nested-fields.mlg >/dev/null
"${CARGO[@]}" run --bin mlg -- build examples/nested-fields.mlg -o target/mallang/nested-fields >/dev/null
nested_fields_output="$(target/mallang/nested-fields)"
if [[ "$nested_fields_output" != $'kim\nlee\n30' ]]; then
  echo "nested fields native build smoke failed: expected kim, lee, and 30, got '$nested_fields_output'" >&2
  exit 1
fi
"${CARGO[@]}" run --bin mlg -- check examples/return-completeness.mlg >/dev/null
"${CARGO[@]}" run --bin mlg -- build examples/return-completeness.mlg -o target/mallang/return-completeness >/dev/null
return_completeness_output="$(target/mallang/return-completeness)"
if [[ "$return_completeness_output" != $'1\n2' ]]; then
  echo "return completeness native build smoke failed: expected 1 and 2, got '$return_completeness_output'" >&2
  exit 1
fi
"${CARGO[@]}" run --bin mlg -- check examples/else-if.mlg >/dev/null
"${CARGO[@]}" run --bin mlg -- build examples/else-if.mlg -o target/mallang/else-if >/dev/null
else_if_output="$(target/mallang/else-if)"
if [[ "$else_if_output" != $'1\n2\n3' ]]; then
  echo "else-if native build smoke failed: expected 1, 2, and 3, got '$else_if_output'" >&2
  exit 1
fi
"${CARGO[@]}" run --bin mlg -- check examples/match-statement.mlg >/dev/null
"${CARGO[@]}" run --bin mlg -- build examples/match-statement.mlg -o target/mallang/match-statement >/dev/null
match_statement_output="$(target/mallang/match-statement)"
if [[ "$match_statement_output" != $'7\n0' ]]; then
  echo "match statement native build smoke failed: expected 7 and 0, got '$match_statement_output'" >&2
  exit 1
fi
