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
"${CARGO[@]}" run --bin mlg -- lex examples/hello.mlg >/dev/null
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
"${CARGO[@]}" run --bin mlg -- check examples/int-division.mlg >/dev/null
"${CARGO[@]}" run --bin mlg -- build examples/int-division.mlg -o target/mallang/int-division >/dev/null
int_division_output="$(target/mallang/int-division)"
if [[ "$int_division_output" != $'3\n2\n6' ]]; then
  echo "int division native build smoke failed: expected 3, 2, 6 got '$int_division_output'" >&2
  exit 1
fi
division_fail_source="target/mallang/run-division-fail.mlg"
cat >"$division_fail_source" <<'MLG'
func main() {
    value := 10
    divisor := 0
    print(value / divisor)
}
MLG
if "${CARGO[@]}" run --bin mlg -- run "$division_fail_source" >/dev/null 2>&1; then
  echo "native division failure smoke failed: expected non-zero exit" >&2
  exit 1
fi
remainder_fail_source="target/mallang/run-remainder-fail.mlg"
cat >"$remainder_fail_source" <<'MLG'
func main() {
    value := 10
    divisor := 0
    print(value % divisor)
}
MLG
if "${CARGO[@]}" run --bin mlg -- run "$remainder_fail_source" >/dev/null 2>&1; then
  echo "native remainder failure smoke failed: expected non-zero exit" >&2
  exit 1
fi
"${CARGO[@]}" run --bin mlg -- check examples/checked-arithmetic.mlg >/dev/null
"${CARGO[@]}" run --bin mlg -- build examples/checked-arithmetic.mlg -o target/mallang/checked-arithmetic >/dev/null
checked_arithmetic_output="$(target/mallang/checked-arithmetic)"
if [[ "$checked_arithmetic_output" != $'42\n38\n80\n-2\n20\n1' ]]; then
  echo "checked arithmetic native build smoke failed: expected checked arithmetic output, got '$checked_arithmetic_output'" >&2
  exit 1
fi
checked_add_fail_source="target/mallang/run-checked-add-fail.mlg"
cat >"$checked_add_fail_source" <<'MLG'
func main() {
    value := 9223372036854775807
    one := 1
    print(value + one)
}
MLG
if "${CARGO[@]}" run --bin mlg -- run "$checked_add_fail_source" >/dev/null 2>&1; then
  echo "native checked add failure smoke failed: expected non-zero exit" >&2
  exit 1
fi
checked_sub_fail_source="target/mallang/run-checked-sub-fail.mlg"
cat >"$checked_sub_fail_source" <<'MLG'
func main() {
    value := -9223372036854775807
    two := 2
    print(value - two)
}
MLG
if "${CARGO[@]}" run --bin mlg -- run "$checked_sub_fail_source" >/dev/null 2>&1; then
  echo "native checked subtract failure smoke failed: expected non-zero exit" >&2
  exit 1
fi
checked_mul_fail_source="target/mallang/run-checked-mul-fail.mlg"
cat >"$checked_mul_fail_source" <<'MLG'
func main() {
    value := 3037000500
    print(value * value)
}
MLG
if "${CARGO[@]}" run --bin mlg -- run "$checked_mul_fail_source" >/dev/null 2>&1; then
  echo "native checked multiply failure smoke failed: expected non-zero exit" >&2
  exit 1
fi
checked_neg_fail_source="target/mallang/run-checked-neg-fail.mlg"
cat >"$checked_neg_fail_source" <<'MLG'
func main() {
    value := -9223372036854775807 - 1
    print(-value)
}
MLG
if "${CARGO[@]}" run --bin mlg -- run "$checked_neg_fail_source" >/dev/null 2>&1; then
  echo "native checked negation failure smoke failed: expected non-zero exit" >&2
  exit 1
fi
checked_div_fail_source="target/mallang/run-checked-div-fail.mlg"
cat >"$checked_div_fail_source" <<'MLG'
func main() {
    value := -9223372036854775807 - 1
    divisor := -1
    print(value / divisor)
}
MLG
if "${CARGO[@]}" run --bin mlg -- run "$checked_div_fail_source" >/dev/null 2>&1; then
  echo "native checked division overflow failure smoke failed: expected non-zero exit" >&2
  exit 1
fi
checked_rem_fail_source="target/mallang/run-checked-rem-fail.mlg"
cat >"$checked_rem_fail_source" <<'MLG'
func main() {
    value := -9223372036854775807 - 1
    divisor := -1
    print(value % divisor)
}
MLG
if "${CARGO[@]}" run --bin mlg -- run "$checked_rem_fail_source" >/dev/null 2>&1; then
  echo "native checked remainder overflow failure smoke failed: expected non-zero exit" >&2
  exit 1
fi
recursive_struct_fail_source="target/mallang/check-recursive-struct-fail.mlg"
cat >"$recursive_struct_fail_source" <<'MLG'
type Node struct {
    next Node
}

func main() {}
MLG
if "${CARGO[@]}" run --bin mlg -- check "$recursive_struct_fail_source" >/dev/null 2>&1; then
  echo "recursive struct check failure smoke failed: expected non-zero exit" >&2
  exit 1
fi
wrapped_recursive_struct_fail_source="target/mallang/check-wrapped-recursive-struct-fail.mlg"
cat >"$wrapped_recursive_struct_fail_source" <<'MLG'
type Node struct {
    next Option[Node]
}

func main() {}
MLG
if "${CARGO[@]}" run --bin mlg -- check "$wrapped_recursive_struct_fail_source" >/dev/null 2>&1; then
  echo "wrapped recursive struct check failure smoke failed: expected non-zero exit" >&2
  exit 1
fi
"${CARGO[@]}" run --bin mlg -- check examples/logical-operators.mlg >/dev/null
"${CARGO[@]}" run --bin mlg -- build examples/logical-operators.mlg -o target/mallang/logical-operators >/dev/null
logical_operators_output="$(target/mallang/logical-operators)"
if [[ "$logical_operators_output" != $'false\n0\ntrue\n0\ntrue\n1\ntrue\n2\ntrue\ntrue\nfalse\ntrue\nfalse' ]]; then
  echo "logical operators native build smoke failed: expected short-circuit bool output, got '$logical_operators_output'" >&2
  exit 1
fi
"${CARGO[@]}" run --bin mlg -- check examples/pipeline.mlg >/dev/null
"${CARGO[@]}" run --bin mlg -- build examples/pipeline.mlg -o target/mallang/pipeline >/dev/null
pipeline_output="$(target/mallang/pipeline)"
if [[ "$pipeline_output" != $'15\nmallang' ]]; then
  echo "pipeline native build smoke failed: expected 15 and mallang, got '$pipeline_output'" >&2
  exit 1
fi
"${CARGO[@]}" run --bin mlg -- check examples/if-statement.mlg >/dev/null
"${CARGO[@]}" run --bin mlg -- build examples/if-statement.mlg -o target/mallang/if-statement >/dev/null
if_statement_output="$(target/mallang/if-statement)"
if [[ "$if_statement_output" != "then" ]]; then
  echo "if statement native build smoke failed: expected then, got '$if_statement_output'" >&2
  exit 1
fi
"${CARGO[@]}" run --bin mlg -- check examples/for-loop.mlg >/dev/null
"${CARGO[@]}" run --bin mlg -- build examples/for-loop.mlg -o target/mallang/for-loop >/dev/null
for_loop_output="$(target/mallang/for-loop)"
if [[ "$for_loop_output" != $'1\n2\n3\n4' ]]; then
  echo "for loop native build smoke failed: expected 1, 2, 3, 4 got '$for_loop_output'" >&2
  exit 1
fi
"${CARGO[@]}" run --bin mlg -- check examples/loop-control.mlg >/dev/null
"${CARGO[@]}" run --bin mlg -- build examples/loop-control.mlg -o target/mallang/loop-control >/dev/null
loop_control_output="$(target/mallang/loop-control)"
if [[ "$loop_control_output" != $'1\n3\n4\n5' ]]; then
  echo "loop control native build smoke failed: expected 1, 3, 4, 5 got '$loop_control_output'" >&2
  exit 1
fi
"${CARGO[@]}" run --bin mlg -- check examples/for-clause.mlg >/dev/null
"${CARGO[@]}" run --bin mlg -- build examples/for-clause.mlg -o target/mallang/for-clause >/dev/null
for_clause_output="$(target/mallang/for-clause)"
if [[ "$for_clause_output" != "8" ]]; then
  echo "for-clause native build smoke failed: expected 8 got '$for_clause_output'" >&2
  exit 1
fi
"${CARGO[@]}" run --bin mlg -- check examples/for-clause-initless.mlg >/dev/null
"${CARGO[@]}" run --bin mlg -- build examples/for-clause-initless.mlg -o target/mallang/for-clause-initless >/dev/null
for_clause_initless_output="$(target/mallang/for-clause-initless)"
if [[ "$for_clause_initless_output" != $'8\n5' ]]; then
  echo "initless for-clause native build smoke failed: expected 8 and 5 got '$for_clause_initless_output'" >&2
  exit 1
fi
"${CARGO[@]}" run --bin mlg -- check examples/for-empty-condition.mlg >/dev/null
"${CARGO[@]}" run --bin mlg -- build examples/for-empty-condition.mlg -o target/mallang/for-empty-condition >/dev/null
for_empty_condition_output="$(target/mallang/for-empty-condition)"
if [[ "$for_empty_condition_output" != $'8\n5\nonce' ]]; then
  echo "for empty condition native build smoke failed: expected 8, 5, once got '$for_empty_condition_output'" >&2
  exit 1
fi
"${CARGO[@]}" run --bin mlg -- check examples/arrays.mlg >/dev/null
"${CARGO[@]}" run --bin mlg -- ir examples/arrays.mlg >/dev/null
"${CARGO[@]}" run --bin mlg -- build examples/arrays.mlg -o target/mallang/arrays >/dev/null
arrays_output="$(target/mallang/arrays)"
if [[ "$arrays_output" != "20" ]]; then
  echo "arrays native build smoke failed: expected 20, got '$arrays_output'" >&2
  exit 1
fi
"${CARGO[@]}" run --bin mlg -- check examples/range-blank.mlg >/dev/null
"${CARGO[@]}" run --bin mlg -- build examples/range-blank.mlg -o target/mallang/range-blank >/dev/null
range_blank_output="$(target/mallang/range-blank)"
if [[ "$range_blank_output" != "6" ]]; then
  echo "range blank native build smoke failed: expected 6, got '$range_blank_output'" >&2
  exit 1
fi
"${CARGO[@]}" run --bin mlg -- check examples/range-index.mlg >/dev/null
"${CARGO[@]}" run --bin mlg -- build examples/range-index.mlg -o target/mallang/range-index >/dev/null
range_index_output="$(target/mallang/range-index)"
if [[ "$range_index_output" != "6" ]]; then
  echo "range index native build smoke failed: expected 6, got '$range_index_output'" >&2
  exit 1
fi
range_index_run_output="$("${CARGO[@]}" run --bin mlg -- run examples/range-index.mlg)"
if [[ "$range_index_run_output" != "6" ]]; then
  echo "range index native run smoke failed: expected 6, got '$range_index_run_output'" >&2
  exit 1
fi
runtime_fail_source="target/mallang/run-bounds-fail.mlg"
cat >"$runtime_fail_source" <<'MLG'
func main() {
    values := [1]int{1}
    i := 1
    print(values[i])
}
MLG
if "${CARGO[@]}" run --bin mlg -- run "$runtime_fail_source" >/dev/null 2>&1; then
  echo "native run failure smoke failed: expected non-zero exit" >&2
  exit 1
fi
"${CARGO[@]}" run --bin mlg -- check examples/non-copy-array-assignment.mlg >/dev/null
"${CARGO[@]}" run --bin mlg -- build examples/non-copy-array-assignment.mlg -o target/mallang/non-copy-array-assignment >/dev/null
non_copy_array_assignment_output="$(target/mallang/non-copy-array-assignment)"
if [[ "$non_copy_array_assignment_output" != "park" ]]; then
  echo "non-copy array assignment native build smoke failed: expected park, got '$non_copy_array_assignment_output'" >&2
  exit 1
fi
"${CARGO[@]}" run --bin mlg -- check examples/array-for-post.mlg >/dev/null
"${CARGO[@]}" run --bin mlg -- build examples/array-for-post.mlg -o target/mallang/array-for-post >/dev/null
array_for_post_output="$(target/mallang/array-for-post)"
if [[ "$array_for_post_output" != "6" ]]; then
  echo "array for-post native build smoke failed: expected 6, got '$array_for_post_output'" >&2
  exit 1
fi
"${CARGO[@]}" run --bin mlg -- check examples/for-clause-prelude.mlg >/dev/null
"${CARGO[@]}" run --bin mlg -- build examples/for-clause-prelude.mlg -o target/mallang/for-clause-prelude >/dev/null
for_clause_prelude_output="$(target/mallang/for-clause-prelude)"
if [[ "$for_clause_prelude_output" != "6" ]]; then
  echo "for-clause prelude native build smoke failed: expected 6, got '$for_clause_prelude_output'" >&2
  exit 1
fi
"${CARGO[@]}" run --bin mlg -- check examples/string-equality.mlg >/dev/null
"${CARGO[@]}" run --bin mlg -- build examples/string-equality.mlg -o target/mallang/string-equality >/dev/null
string_equality_output="$(target/mallang/string-equality)"
if [[ "$string_equality_output" != $'same\nmallang' ]]; then
  echo "string equality native build smoke failed: expected same and mallang, got '$string_equality_output'" >&2
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
"${CARGO[@]}" run --bin mlg -- check examples/print-adt.mlg >/dev/null
"${CARGO[@]}" run --bin mlg -- build examples/print-adt.mlg -o target/mallang/print-adt >/dev/null
print_adt_output="$(target/mallang/print-adt)"
if [[ "$print_adt_output" != $'Some(7)\nNone\nOk(1)\nErr(bad)\nSome(Ok(9))' ]]; then
  echo "ADT print native build smoke failed: expected ADT display output, got '$print_adt_output'" >&2
  exit 1
fi
"${CARGO[@]}" run --bin mlg -- check examples/match-temp.mlg >/dev/null
"${CARGO[@]}" run --bin mlg -- build examples/match-temp.mlg -o target/mallang/match-temp >/dev/null
match_temp_output="$(target/mallang/match-temp)"
if [[ "$match_temp_output" != "0" ]]; then
  echo "match temp native build smoke failed: expected 0, got '$match_temp_output'" >&2
  exit 1
fi
"${CARGO[@]}" run --bin mlg -- check examples/if-match-expression.mlg >/dev/null
"${CARGO[@]}" run --bin mlg -- build examples/if-match-expression.mlg -o target/mallang/if-match-expression >/dev/null
if_match_expression_output="$(target/mallang/if-match-expression)"
if [[ "$if_match_expression_output" != $'7\n0' ]]; then
  echo "if match expression native build smoke failed: expected 7 and 0, got '$if_match_expression_output'" >&2
  exit 1
fi
"${CARGO[@]}" run --bin mlg -- check examples/match-arm-prelude.mlg >/dev/null
"${CARGO[@]}" run --bin mlg -- build examples/match-arm-prelude.mlg -o target/mallang/match-arm-prelude >/dev/null
match_arm_prelude_output="$(target/mallang/match-arm-prelude)"
if [[ "$match_arm_prelude_output" != $'7\n0' ]]; then
  echo "match arm prelude native build smoke failed: expected 7 and 0, got '$match_arm_prelude_output'" >&2
  exit 1
fi
"${CARGO[@]}" run --bin mlg -- check examples/structs.mlg >/dev/null
"${CARGO[@]}" run --bin mlg -- build examples/structs.mlg -o target/mallang/structs >/dev/null
structs_output="$(target/mallang/structs)"
if [[ "$structs_output" != $'kim\n30' ]]; then
  echo "structs native build smoke failed: expected kim and 30, got '$structs_output'" >&2
  exit 1
fi
"${CARGO[@]}" run --bin mlg -- check examples/print-struct.mlg >/dev/null
"${CARGO[@]}" run --bin mlg -- build examples/print-struct.mlg -o target/mallang/print-struct >/dev/null
print_struct_output="$(target/mallang/print-struct)"
if [[ "$print_struct_output" != "User{name: kim, age: 30, active: true, profile: Profile{display: neo}, status: Some(7)}" ]]; then
  echo "struct print native build smoke failed: expected User field display output, got '$print_struct_output'" >&2
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
"${CARGO[@]}" run --bin mlg -- check examples/array-element-borrow.mlg >/dev/null
"${CARGO[@]}" run --bin mlg -- build examples/array-element-borrow.mlg -o target/mallang/array-element-borrow >/dev/null
array_element_borrow_output="$(target/mallang/array-element-borrow)"
if [[ "$array_element_borrow_output" != $'kim\npark' ]]; then
  echo "array element borrow native build smoke failed: expected kim and park, got '$array_element_borrow_output'" >&2
  exit 1
fi
"${CARGO[@]}" run --bin mlg -- check examples/array-element-methods.mlg >/dev/null
"${CARGO[@]}" run --bin mlg -- build examples/array-element-methods.mlg -o target/mallang/array-element-methods >/dev/null
array_element_methods_output="$(target/mallang/array-element-methods)"
if [[ "$array_element_methods_output" != "3" ]]; then
  echo "array element methods native build smoke failed: expected 3, got '$array_element_methods_output'" >&2
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
