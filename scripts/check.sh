#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

scripts/check-agent-harness-interface.sh
python3 scripts/check-todo-state.py
scripts/check-work-packet-lifecycle.sh
scripts/check-self-hosting-gate-interface.sh
python3 scripts/check-v1-conformance.py
scripts/check-self-hosting-bootstrap.sh
scripts/check-self-hosting-lexer.sh
scripts/check-self-hosting-backend.sh --assume-bootstrap

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
CLANG_BIN="${CLANG:-clang}"

expect_native_runtime_failure() {
  local label="$1"
  local source="$2"
  local expected_stderr="$3"
  local stderr_path="target/mallang/${label}.stderr"

  if "${CARGO[@]}" run --bin mlg -- run "$source" >/dev/null 2>"$stderr_path"; then
    echo "native $label failure smoke failed: expected non-zero exit" >&2
    exit 1
  fi

  if ! grep -Fq "$expected_stderr" "$stderr_path"; then
    echo "native $label failure smoke failed: expected stderr containing '$expected_stderr'" >&2
    echo "stderr was:" >&2
    cat "$stderr_path" >&2
    exit 1
  fi
}

expect_check_failure() {
  local label="$1"
  local source="$2"
  local expected_stderr="$3"
  local stdout_path="target/mallang/${label}.stdout"
  local stderr_path="target/mallang/${label}.stderr"

  if "${CARGO[@]}" run --quiet --bin mlg -- check "$source" >"$stdout_path" 2>"$stderr_path"; then
    echo "$label check failure smoke failed: expected non-zero exit" >&2
    exit 1
  fi
  if [[ -s "$stdout_path" ]]; then
    echo "$label check failure smoke failed: expected empty stdout" >&2
    exit 1
  fi
  if ! grep -Fq "$expected_stderr" "$stderr_path" || ! grep -Fq "$source:" "$stderr_path"; then
    echo "$label check failure smoke failed: missing diagnostic or source location" >&2
    cat "$stderr_path" >&2
    exit 1
  fi
}

expect_sanitized_native_output() {
  local label="$1"
  local c_source="$2"
  local expected_stdout="$3"
  local binary_path="target/mallang/${label}-san"
  local stderr_path="target/mallang/${label}-san.stderr"
  local output

  if ! "$CLANG_BIN" -fsanitize=address,undefined -fno-omit-frame-pointer "$c_source" -o "$binary_path" 2>"$stderr_path"; then
    echo "sanitizer $label compile failed" >&2
    cat "$stderr_path" >&2
    exit 1
  fi
  if [[ -s "$stderr_path" ]]; then
    echo "sanitizer $label compile emitted stderr" >&2
    cat "$stderr_path" >&2
    exit 1
  fi

  if ! output="$("$binary_path" 2>"$stderr_path")"; then
    echo "sanitizer $label run failed" >&2
    cat "$stderr_path" >&2
    exit 1
  fi
  if [[ "$output" != "$expected_stdout" ]]; then
    echo "sanitizer $label output mismatch: expected '$expected_stdout', got '$output'" >&2
    exit 1
  fi
  if [[ -s "$stderr_path" ]]; then
    echo "sanitizer $label emitted stderr" >&2
    cat "$stderr_path" >&2
    exit 1
  fi
}

expect_warning_clean_generated_c() {
  local label="$1"
  local c_source="$2"
  local binary_path="target/mallang/${label}-warn"
  local stderr_path="target/mallang/${label}-warn.stderr"

  if ! "$CLANG_BIN" -std=c11 -Wall -Wextra -Werror "$c_source" -o "$binary_path" 2>"$stderr_path"; then
    echo "warning-clean $label compile failed" >&2
    cat "$stderr_path" >&2
    exit 1
  fi
  if [[ -s "$stderr_path" ]]; then
    echo "warning-clean $label compile emitted stderr" >&2
    cat "$stderr_path" >&2
    exit 1
  fi
}

generated_c_smoke_labels() {
  sed -nE 's#.*build examples/[A-Za-z0-9_.-]+\.mlg -o target/mallang/([^ ]+).*#\1#p' scripts/check.sh |
    sort -u
}

expect_all_warning_clean_generated_c() {
  local label
  while IFS= read -r label; do
    expect_warning_clean_generated_c "$label" "target/mallang/${label}.c"
  done < <(generated_c_smoke_labels)
}

"${CARGO[@]}" fmt --all --check
"${CARGO[@]}" test --workspace
"${CARGO[@]}" clippy --workspace --all-targets -- -D warnings
"${CARGO[@]}" build --locked --bin mlg
scripts/build-self-hosted-compiler.sh \
  --stage0 target/debug/mlg \
  --output target/debug/mlgc \
  >/dev/null
scripts/check-naming-lint.sh target/debug/mlg target/debug/mlgc
crate_version="$(sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml)"
version_output="$("${CARGO[@]}" run --quiet --bin mlg -- --version)"
if [[ "$version_output" != "mlg $crate_version" ]]; then
  echo "version smoke failed: expected mlg $crate_version, got '$version_output'" >&2
  exit 1
fi
mkdir -p target/mallang
help_stderr="target/mallang/help.stderr"
help_output="$("${CARGO[@]}" run --quiet --bin mlg -- --help 2>"$help_stderr")"
if [[ "$help_output" != *"usage:"* || \
  "$help_output" != *"target/debug/mlg check <input>"* || \
  "$help_output" != *"target/debug/mlg lint [--allow <rule-id>] [--deny-warnings] <input>"* || \
  "$help_output" != *"target/debug/mlg ir <input>"* || \
  "$help_output" != *"target/debug/mlg fmt [--check] <input>"* || \
  "$help_output" != *"target/debug/mlg test <input> [--exact <test-id>]"* || \
  "$help_output" != *"target/debug/mlg --version"* ]]; then
  echo "help smoke failed: unexpected help output '$help_output'" >&2
  exit 1
fi
if [[ -s "$help_stderr" ]]; then
  echo "help smoke failed: expected empty stderr" >&2
  cat "$help_stderr" >&2
  exit 1
fi
scripts/check-formatter.sh target/debug/mlg
scripts/check-test-workflow.sh target/debug/mlg
scripts/check-path-dependencies.sh target/debug/mlg
scripts/check-diagnostics.sh target/debug/mlg
scripts/check-parser-recovery.sh target/debug/mlg
scripts/check-v1-migration.sh target/debug/mlg
scripts/check-hardening-corpus.sh target/debug/mlg
scripts/check-v08-reproducibility.sh --skip-release-archive target/debug/mlg
scripts/check-v07-acceptance.sh
scripts/check-v09-dogfood.sh \
  --compiler target/mallang/release-artifact-smoke/home/.local/bin/mlg
no_args_stdout="target/mallang/no-args.stdout"
no_args_stderr="target/mallang/no-args.stderr"
if "${CARGO[@]}" run --quiet --bin mlg -- >"$no_args_stdout" 2>"$no_args_stderr"; then
  echo "no-args smoke failed: expected non-zero exit" >&2
  exit 1
fi
if [[ -s "$no_args_stdout" || ! -s "$no_args_stderr" ]]; then
  echo "no-args smoke failed: expected usage on stderr only" >&2
  exit 1
fi
unknown_stdout="target/mallang/unknown-command.stdout"
unknown_stderr="target/mallang/unknown-command.stderr"
if "${CARGO[@]}" run --quiet --bin mlg -- nope >"$unknown_stdout" 2>"$unknown_stderr"; then
  echo "unknown-command smoke failed: expected non-zero exit" >&2
  exit 1
fi
if [[ -s "$unknown_stdout" ]] || ! grep -Fq 'unknown subcommand `nope`' "$unknown_stderr"; then
  echo "unknown-command smoke failed: expected diagnostic on stderr only" >&2
  exit 1
fi
missing_example_smokes="$(
  comm -23 \
    <(find examples -maxdepth 1 -type f -name '*.mlg' | sort) \
    <(grep -Eo 'examples/[A-Za-z0-9_.-]+\.mlg' scripts/check.sh | sort -u)
)"
if [[ -n "$missing_example_smokes" ]]; then
  echo "example smoke coverage failed: examples missing from scripts/check.sh" >&2
  echo "$missing_example_smokes" >&2
  exit 1
fi
standard_fixture="tests/fixtures/v06-standard-registry/standard-intrinsics.mlg"
"${CARGO[@]}" run --quiet --bin mlg -- check "$standard_fixture" >/dev/null
standard_ir="$("${CARGO[@]}" run --quiet --bin mlg -- ir "$standard_fixture")"
for expected_intrinsic in StringsByteLen CollectionsNewMap CollectionsCount; do
  encoded_intrinsic="$(
    python3 -c \
      'import sys; print(",".join(str(byte) for byte in sys.argv[1].encode("utf-8")))' \
      "$expected_intrinsic"
  )"
  if ! grep -F "|E|Expr.IntrinsicCall|" <<<"$standard_ir" |
    grep -Fq "|$encoded_intrinsic|"; then
    echo "standard registry IR smoke failed: typed intrinsic identity is missing: $expected_intrinsic" >&2
    exit 1
  fi
done
standard_build_stdout="target/mallang/v06-standard-registry.stdout"
standard_build_stderr="target/mallang/v06-standard-registry.stderr"
"${CARGO[@]}" run --quiet --bin mlg -- build "$standard_fixture" -o target/mallang/v06-standard-registry >"$standard_build_stdout" 2>"$standard_build_stderr"
if [[ "$(cat "$standard_build_stdout")" != "target/mallang/v06-standard-registry" ]] || [[ -s "$standard_build_stderr" ]]; then
  echo "standard registry native build smoke failed" >&2
  cat "$standard_build_stdout" >&2
  cat "$standard_build_stderr" >&2
  exit 1
fi
standard_registry_output="$(target/mallang/v06-standard-registry)"
if [[ "$standard_registry_output" != $'7\n0' ]]; then
  echo "standard registry native output mismatch: got '$standard_registry_output'" >&2
  exit 1
fi
expect_warning_clean_generated_c "v06-standard-registry" "target/mallang/standard-intrinsics.c"
standard_unused_fixture="tests/fixtures/v06-standard-registry/unused-standard-imports.mlg"
"${CARGO[@]}" run --quiet --bin mlg -- check "$standard_unused_fixture" >/dev/null
"${CARGO[@]}" run --quiet --bin mlg -- build "$standard_unused_fixture" -o target/mallang/v06-standard-unused >/dev/null
standard_unused_output="$(target/mallang/v06-standard-unused)"
if [[ "$standard_unused_output" != "1" ]]; then
  echo "unused standard import native smoke failed: expected 1, got '$standard_unused_output'" >&2
  exit 1
fi
expect_warning_clean_generated_c "v06-standard-unused" "target/mallang/unused-standard-imports.c"
"${CARGO[@]}" run --quiet --bin mlg -- check examples/standard-strings.mlg >/dev/null
"${CARGO[@]}" run --quiet --bin mlg -- build examples/standard-strings.mlg -o target/mallang/standard-strings >/dev/null
standard_strings_output="$(target/mallang/standard-strings)"
if [[ "$standard_strings_output" != $'8\n3\n8\ntrue\n1\na||b|\n가/a\n-42\ntrue\n-9223372036854775808\nInvalidData\nfalse' ]]; then
  echo "standard strings native build smoke output mismatch: got '$standard_strings_output'" >&2
  exit 1
fi
scripts/check-standard-strings-runtime.sh target/mallang/standard-strings.c
standard_strings_edge_fixture="tests/fixtures/v06-standard-strings/edge-cases.mlg"
"${CARGO[@]}" run --quiet --bin mlg -- check "$standard_strings_edge_fixture" >/dev/null
"${CARGO[@]}" run --quiet --bin mlg -- build "$standard_strings_edge_fixture" -o target/mallang/v06-standard-strings-edge >/dev/null
standard_strings_edge_output="$(target/mallang/v06-standard-strings-edge)"
if [[ "$standard_strings_edge_output" != $'0\n-1\n0\n1\n|a||b|\n9223372036854775807\n-9223372036854775808\nInvalidData\nInvalidData\nInvalidData\nInvalidData\ntrue\nfalse\nInvalidData\nError{kind: InvalidData, message: invalid integer text}\n-9223372036854775808\nfalse' ]]; then
  echo "standard strings edge-case smoke output mismatch: got '$standard_strings_edge_output'" >&2
  exit 1
fi
expect_warning_clean_generated_c "v06-standard-strings-edge" "target/mallang/edge-cases.c"
self_hosting_cursor_fixture="tests/fixtures/self-hosting/string-cursor.mlg"
"${CARGO[@]}" run --quiet --bin mlg -- check "$self_hosting_cursor_fixture" >/dev/null
"${CARGO[@]}" run --quiet --bin mlg -- build "$self_hosting_cursor_fixture" -o target/mallang/self-hosting-string-cursor >/dev/null
self_hosting_cursor_output="$(target/mallang/self-hosting-string-cursor)"
if [[ "$self_hosting_cursor_output" != $'65\n234\n128\n90\nInvalidInput\nInvalidInput\nA\n가\nZ\n0\nInvalidInput\nInvalidInput\nInvalidInput\nInvalidInput\nInvalidInput' ]]; then
  echo "self-hosting string cursor output mismatch: got '$self_hosting_cursor_output'" >&2
  exit 1
fi
scripts/check-self-hosting-string-cursor.sh target/mallang/string-cursor.c
"${CARGO[@]}" run --quiet --bin mlg -- check examples/process-io.mlg >/dev/null
"${CARGO[@]}" run --quiet --bin mlg -- build examples/process-io.mlg -o target/mallang/process-io >/dev/null
scripts/check-process-io-runtime.sh
"${CARGO[@]}" run --quiet --bin mlg -- check examples/file-io.mlg >/dev/null
"${CARGO[@]}" run --quiet --bin mlg -- build examples/file-io.mlg -o target/mallang/file-io >/dev/null
scripts/check-file-io-runtime.sh
scripts/check-streaming-file-io-runtime.sh
"${CARGO[@]}" run --quiet --bin mlg -- check examples/collections-map.mlg >/dev/null
"${CARGO[@]}" run --quiet --bin mlg -- build examples/collections-map.mlg -o target/mallang/collections-map >/dev/null
collections_map_output="$(target/mallang/collections-map)"
if [[ "$collections_map_output" != $'inserted\n1\n1\nKim\n2\ntrue\ntrue\nKim\n3\ntrue\nfalse\nKim\n3\n0' ]]; then
  echo "collections Map native build smoke output mismatch: got '$collections_map_output'" >&2
  exit 1
fi
collections_growth_fixture="tests/fixtures/v06-collections-map/growth-and-ownership.mlg"
"${CARGO[@]}" run --quiet --bin mlg -- check "$collections_growth_fixture" >/dev/null
"${CARGO[@]}" run --quiet --bin mlg -- build "$collections_growth_fixture" -o target/mallang/v06-collections-map-growth >/dev/null
collections_growth_output="$(target/mallang/v06-collections-map-growth)"
if [[ "$collections_growth_output" != $'24\n17\ntrue\n3\n23\ntrue\n11\ntrue\n20\ntrue\n7\ntrue\n7\n8\n0' ]]; then
  echo "collections Map growth native output mismatch: got '$collections_growth_output'" >&2
  exit 1
fi
scripts/check-collections-map-runtime.sh \
  target/mallang/collections-map.c \
  target/mallang/growth-and-ownership.c
"${CARGO[@]}" run --quiet --bin mlg -- check examples/projects/textstats >/dev/null
"${CARGO[@]}" run --quiet --bin mlg -- build examples/projects/textstats -o target/mallang/textstats >/dev/null
scripts/check-reference-cli.sh \
  examples/projects/textstats/target/mallang/textstats.c \
  target/mallang/textstats \
  tests/fixtures/v06-reference-cli/input.txt
"${CARGO[@]}" run --bin mlg -- lex examples/hello.mlg >/dev/null
"${CARGO[@]}" run --quiet --bin mlg -- check examples/hello.mlg >/dev/null
"${CARGO[@]}" run --quiet --bin mlg -- build examples/hello.mlg -o target/mallang/hello >/dev/null
hello_output="$(target/mallang/hello)"
if [[ "$hello_output" != $'hello\nkim' ]]; then
  echo "hello native build smoke failed: expected hello and kim, got '$hello_output'" >&2
  exit 1
fi
"${CARGO[@]}" run --bin mlg -- parse examples/first.mlg >/dev/null
"${CARGO[@]}" run --bin mlg -- check examples/first.mlg >/dev/null
"${CARGO[@]}" run --bin mlg -- build examples/first.mlg -o target/mallang/first >/dev/null
first_output="$(target/mallang/first)"
if [[ "$first_output" != "30" ]]; then
  echo "first native build smoke failed: expected 30, got '$first_output'" >&2
  exit 1
fi
"${CARGO[@]}" run --bin mlg -- check examples/function-values.mlg >/dev/null
"${CARGO[@]}" run --bin mlg -- build examples/function-values.mlg -o target/mallang/function-values >/dev/null
function_values_output="$(target/mallang/function-values)"
if [[ "$function_values_output" != $'20\n22\n42' ]]; then
  echo "function value native build smoke failed: expected 20, 22, 42 got '$function_values_output'" >&2
  exit 1
fi
"${CARGO[@]}" run --bin mlg -- check examples/closures.mlg >/dev/null
"${CARGO[@]}" run --bin mlg -- build examples/closures.mlg -o target/mallang/closures >/dev/null
closures_output="$(target/mallang/closures)"
if [[ "$closures_output" != $'12\n15\n7\n7' ]]; then
  echo "closure native build smoke failed: expected 12, 15, 7, 7 got '$closures_output'" >&2
  exit 1
fi
"${CARGO[@]}" run --bin mlg -- check examples/mutable-closures.mlg >/dev/null
"${CARGO[@]}" run --bin mlg -- build examples/mutable-closures.mlg -o target/mallang/mutable-closures >/dev/null
mutable_closures_output="$(target/mallang/mutable-closures)"
if [[ "$mutable_closures_output" != $'1\n3\n7\n8\n9\n10\n11\n10' ]]; then
  echo "mutable closure native build smoke failed: expected 1, 3, 7, 8, 9, 10, 11, 10 got '$mutable_closures_output'" >&2
  exit 1
fi
"${CARGO[@]}" run --bin mlg -- check examples/nested-closures.mlg >/dev/null
"${CARGO[@]}" run --bin mlg -- build examples/nested-closures.mlg -o target/mallang/nested-closures >/dev/null
nested_closures_output="$(target/mallang/nested-closures)"
if [[ "$nested_closures_output" != $'17\n9\n16\n20\n17' ]]; then
  echo "nested closure native build smoke failed: expected 17, 9, 16, 20, 17 got '$nested_closures_output'" >&2
  exit 1
fi
"${CARGO[@]}" run --bin mlg -- check examples/generics.mlg >/dev/null
"${CARGO[@]}" run --bin mlg -- build examples/generics.mlg -o target/mallang/generics >/dev/null
generics_output="$(target/mallang/generics)"
if [[ "$generics_output" != $'7\nmallang\n11\npair\n3\nupdated' ]]; then
  echo "generics native build smoke failed: expected 7, mallang, 11, pair, 3, updated got '$generics_output'" >&2
  exit 1
fi
"${CARGO[@]}" run --bin mlg -- check examples/generic-enums.mlg >/dev/null
"${CARGO[@]}" run --bin mlg -- build examples/generic-enums.mlg -o target/mallang/generic-enums >/dev/null
generic_enums_output="$(target/mallang/generic-enums)"
if [[ "$generic_enums_output" != $'7\n-1\n0\n9' ]]; then
  echo "generic enum native build smoke failed: expected 7, -1, 0, 9 got '$generic_enums_output'" >&2
  exit 1
fi
"${CARGO[@]}" run --bin mlg -- check examples/recursive-enums.mlg >/dev/null
"${CARGO[@]}" run --bin mlg -- build examples/recursive-enums.mlg -o target/mallang/recursive-enums >/dev/null
recursive_enums_output="$(target/mallang/recursive-enums)"
if [[ "$recursive_enums_output" != $'6\n7' ]]; then
  echo "recursive enum native build smoke failed: expected 6, 7 got '$recursive_enums_output'" >&2
  exit 1
fi
"${CARGO[@]}" run --bin mlg -- check examples/full-expression-cleanup.mlg >/dev/null
"${CARGO[@]}" run --bin mlg -- build examples/full-expression-cleanup.mlg -o target/mallang/full-expression-cleanup >/dev/null
full_expression_cleanup_output="$(target/mallang/full-expression-cleanup)"
if [[ "$full_expression_cleanup_output" != $'42\n3\n5\n10\n0\n1\n3\n1\n2\n3\n17\n2\n1\n5\n4\n0\n1\n7\n20' ]]; then
  echo "full-expression cleanup native build smoke output mismatch: got '$full_expression_cleanup_output'" >&2
  exit 1
fi
"${CARGO[@]}" run --bin mlg -- check examples/string-runtime.mlg >/dev/null
"${CARGO[@]}" run --bin mlg -- build examples/string-runtime.mlg -o target/mallang/string-runtime >/dev/null
string_runtime_output="$(target/mallang/string-runtime)"
if [[ "$string_runtime_output" != $'literal\nreturned\ntrue\nfield\nreplaced\n1\nindexed-after\nenum\nclosure\nmutated' ]]; then
  echo "string runtime native build smoke output mismatch: got '$string_runtime_output'" >&2
  exit 1
fi
scripts/check-string-runtime.sh target/mallang/string-runtime.c
"${CARGO[@]}" run --bin mlg -- check examples/borrow-range-contract.mlg >/dev/null
"${CARGO[@]}" run --bin mlg -- build examples/borrow-range-contract.mlg -o target/mallang/borrow-range-contract >/dev/null
borrow_range_contract_output="$(target/mallang/borrow-range-contract)"
if [[ "$borrow_range_contract_output" != $'kim\n1\nvisited\n2\nlee\n3\nvisited\n4\nvisited\n2' ]]; then
  echo "borrow/range contract native build smoke output mismatch: got '$borrow_range_contract_output'" >&2
  exit 1
fi
"${CARGO[@]}" run --bin mlg -- check examples/allocation-accounting.mlg >/dev/null
"${CARGO[@]}" run --bin mlg -- build examples/allocation-accounting.mlg -o target/mallang/allocation-accounting >/dev/null
allocation_accounting_output="$(target/mallang/allocation-accounting)"
if [[ "$allocation_accounting_output" != $'4\n5\n3\n2\n2\n6' ]]; then
  echo "allocation accounting native build smoke output mismatch: got '$allocation_accounting_output'" >&2
  exit 1
fi
scripts/check-allocation-runtime.sh target/mallang/allocation-accounting.c
expect_check_failure \
  "v05-first-class-borrow" \
  "tests/fixtures/invalid-v05-ownership/first-class-borrow.mlg" \
  'first-class borrow values are not supported; `con` and `mut` are only valid on direct call arguments'
expect_check_failure \
  "v05-mutable-range" \
  "tests/fixtures/invalid-v05-ownership/mutable-range.mlg" \
  'mutable range value bindings are not supported; use indexed assignment or indexed `mut` call access'
expect_check_failure \
  "v05-by-reference-range" \
  "tests/fixtures/invalid-v05-ownership/by-reference-range.mlg" \
  'by-reference range value bindings are not supported; use index-only range and indexed `con` call access'
expect_check_failure \
  "v05-borrowed-return" \
  "tests/fixtures/invalid-v05-ownership/borrowed-return.mlg" \
  'cannot move borrowed value `value`'
expect_check_failure \
  "v05-borrowed-store" \
  "tests/fixtures/invalid-v05-ownership/borrowed-store.mlg" \
  'cannot move borrowed value `value`'
expect_check_failure \
  "v05-borrowed-owned-argument" \
  "tests/fixtures/invalid-v05-ownership/borrowed-owned-argument.mlg" \
  'cannot move borrowed value `value`'
expect_check_failure \
  "v05-use-after-move" \
  "tests/fixtures/invalid-v05-ownership/use-after-move.mlg" \
  'use of moved value `value`'
expect_check_failure \
  "v05-overlapping-borrows" \
  "tests/fixtures/invalid-v05-ownership/overlapping-borrows.mlg" \
  'borrow of `value` overlaps with an active borrow in this call'
expect_check_failure \
  "closure-borrowed-capture" \
  "tests/fixtures/invalid-closures/borrowed-capture.mlg" \
  'cannot capture borrowed non-Copy value `name`'
expect_check_failure \
  "closure-immutable-mutable-capture" \
  "tests/fixtures/invalid-closures/immutable-mutable-capture.mlg" \
  'mutable closure capture `count` requires a mutable source binding'
expect_check_failure \
  "closure-function-use-after-move" \
  "tests/fixtures/invalid-closures/function-use-after-move.mlg" \
  'use of moved value `transform`'
expect_check_failure \
  "closure-mutable-alias" \
  "tests/fixtures/invalid-closures/mutable-alias.mlg" \
  'borrow of `next` overlaps with an active borrow in this call'
expect_check_failure \
  "closure-recursive" \
  "tests/fixtures/invalid-closures/recursive-closure.mlg" \
  'recursive closure `recurse` is not supported in v0.3'
project_input="examples/projects/hello"
"${CARGO[@]}" run --bin mlg -- check "$project_input" >/dev/null
project_output_path="$("${CARGO[@]}" run --quiet --bin mlg -- build "$project_input")"
if [[ "$project_output_path" != */examples/projects/hello/target/mallang/hello ]]; then
  echo "project default build path smoke failed: got '$project_output_path'" >&2
  exit 1
fi
project_output="$("$project_output_path")"
if [[ "$project_output" != $'kim\n42\n22\n15\ngeneric\n8\nupdated\n13\n0' ]]; then
  echo "project native build smoke failed: expected kim, 42, 22, 15, generic, 8, updated, 13, 0 got '$project_output'" >&2
  exit 1
fi
project_run_output="$("${CARGO[@]}" run --quiet --bin mlg -- run "$project_input/mallang.toml")"
if [[ "$project_run_output" != $'kim\n42\n22\n15\ngeneric\n8\nupdated\n13\n0' ]]; then
  echo "project native run smoke failed: expected kim, 42, 22, 15, generic, 8, updated, 13, 0 got '$project_run_output'" >&2
  exit 1
fi
expect_warning_clean_generated_c \
  "project-hello" \
  "$project_input/target/mallang/hello.c"
expect_sanitized_native_output \
  "project-hello" \
  "$project_input/target/mallang/hello.c" \
  $'kim\n42\n22\n15\ngeneric\n8\nupdated\n13\n0'
expect_check_failure \
  "generic-enum-non-exhaustive" \
  "tests/fixtures/invalid-generic-enums/non-exhaustive.mlg" \
  'match is not exhaustive; missing Maybe.Some -> Result.Err'
expect_check_failure \
  "generic-enum-constructor-payload" \
  "tests/fixtures/invalid-generic-enums/constructor-payload.mlg" \
  'payload type mismatch for `Maybe[int].Some`: expected `int`, got `string`'

project_cycle_stderr="target/mallang/project-cycle.stderr"
if "${CARGO[@]}" run --quiet --bin mlg -- check tests/fixtures/project-cycle >/dev/null 2>"$project_cycle_stderr"; then
  echo "project cycle smoke failed: expected non-zero exit" >&2
  exit 1
fi
if ! grep -Fq 'package import cycle: cycle -> cycle/a -> cycle' "$project_cycle_stderr"; then
  echo "project cycle smoke failed: expected cycle diagnostic" >&2
  cat "$project_cycle_stderr" >&2
  exit 1
fi
if ! grep -Fq 'src/a/a.mlg:3:1:' "$project_cycle_stderr"; then
  echo "project cycle smoke failed: expected source location" >&2
  cat "$project_cycle_stderr" >&2
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
expect_native_runtime_failure \
  "division" \
  "$division_fail_source" \
  "mallang runtime error: division by zero"
remainder_fail_source="target/mallang/run-remainder-fail.mlg"
cat >"$remainder_fail_source" <<'MLG'
func main() {
    value := 10
    divisor := 0
    print(value % divisor)
}
MLG
expect_native_runtime_failure \
  "remainder" \
  "$remainder_fail_source" \
  "mallang runtime error: division by zero"
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
expect_native_runtime_failure \
  "checked-add" \
  "$checked_add_fail_source" \
  "mallang runtime error: integer overflow"
checked_sub_fail_source="target/mallang/run-checked-sub-fail.mlg"
cat >"$checked_sub_fail_source" <<'MLG'
func main() {
    value := -9223372036854775807
    two := 2
    print(value - two)
}
MLG
expect_native_runtime_failure \
  "checked-subtract" \
  "$checked_sub_fail_source" \
  "mallang runtime error: integer overflow"
checked_mul_fail_source="target/mallang/run-checked-mul-fail.mlg"
cat >"$checked_mul_fail_source" <<'MLG'
func main() {
    value := 3037000500
    print(value * value)
}
MLG
expect_native_runtime_failure \
  "checked-multiply" \
  "$checked_mul_fail_source" \
  "mallang runtime error: integer overflow"
checked_neg_fail_source="target/mallang/run-checked-neg-fail.mlg"
cat >"$checked_neg_fail_source" <<'MLG'
func main() {
    value := -9223372036854775807 - 1
    print(-value)
}
MLG
expect_native_runtime_failure \
  "checked-negation" \
  "$checked_neg_fail_source" \
  "mallang runtime error: integer overflow"
checked_div_fail_source="target/mallang/run-checked-div-fail.mlg"
cat >"$checked_div_fail_source" <<'MLG'
func main() {
    value := -9223372036854775807 - 1
    divisor := -1
    print(value / divisor)
}
MLG
expect_native_runtime_failure \
  "checked-division-overflow" \
  "$checked_div_fail_source" \
  "mallang runtime error: integer overflow"
checked_rem_fail_source="target/mallang/run-checked-rem-fail.mlg"
cat >"$checked_rem_fail_source" <<'MLG'
func main() {
    value := -9223372036854775807 - 1
    divisor := -1
    print(value % divisor)
}
MLG
expect_native_runtime_failure \
  "checked-remainder-overflow" \
  "$checked_rem_fail_source" \
  "mallang runtime error: integer overflow"
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
main_param_fail_source="target/mallang/check-main-param-fail.mlg"
cat >"$main_param_fail_source" <<'MLG'
func main(value int) {
    print(value)
}
MLG
if "${CARGO[@]}" run --bin mlg -- check "$main_param_fail_source" >/dev/null 2>&1; then
  echo "main parameter check failure smoke failed: expected non-zero exit" >&2
  exit 1
fi
array_print_fail_source="target/mallang/check-array-print-fail.mlg"
cat >"$array_print_fail_source" <<'MLG'
func main() {
    values := [2]int{1, 2}
    print(values)
}
MLG
if "${CARGO[@]}" run --bin mlg -- check "$array_print_fail_source" >/dev/null 2>&1; then
  echo "array print check failure smoke failed: expected non-zero exit" >&2
  exit 1
fi
struct_array_print_fail_source="target/mallang/check-struct-array-print-fail.mlg"
cat >"$struct_array_print_fail_source" <<'MLG'
type Box struct {
    values [1]int
}

func main() {
    box := Box{values: [1]int{1}}
    print(box)
}
MLG
if "${CARGO[@]}" run --bin mlg -- check "$struct_array_print_fail_source" >/dev/null 2>&1; then
  echo "struct array print check failure smoke failed: expected non-zero exit" >&2
  exit 1
fi
print_value_fail_source="target/mallang/check-print-value-fail.mlg"
cat >"$print_value_fail_source" <<'MLG'
func main() {
    value := print(1)
    print(value)
}
MLG
if "${CARGO[@]}" run --bin mlg -- check "$print_value_fail_source" >/dev/null 2>&1; then
  echo "print value-position check failure smoke failed: expected non-zero exit" >&2
  exit 1
fi
builtin_value_name_fail_source="target/mallang/check-builtin-value-name-fail.mlg"
cat >"$builtin_value_name_fail_source" <<'MLG'
func append(value int) int {
    return value
}

func main() {
    print(append(1))
}
MLG
if "${CARGO[@]}" run --bin mlg -- check "$builtin_value_name_fail_source" >/dev/null 2>&1; then
  echo "builtin value name check failure smoke failed: expected non-zero exit" >&2
  exit 1
fi
top_level_name_conflict_fail_source="target/mallang/check-top-level-name-conflict-fail.mlg"
cat >"$top_level_name_conflict_fail_source" <<'MLG'
type User struct {
    age int
}

func User() {
}

func main() {}
MLG
if "${CARGO[@]}" run --bin mlg -- check "$top_level_name_conflict_fail_source" >/dev/null 2>&1; then
  echo "top-level name conflict check failure smoke failed: expected non-zero exit" >&2
  exit 1
fi
"${CARGO[@]}" run --bin mlg -- check examples/shadowing.mlg >/dev/null
"${CARGO[@]}" run --bin mlg -- build examples/shadowing.mlg -o target/mallang/shadowing >/dev/null
shadowing_output="$(target/mallang/shadowing)"
if [[ "$shadowing_output" != $'7\nouter\nkeep\nloop\nrange\nouter\ninner\nouter\nwhile\nouter' ]]; then
  echo "shadowing native build smoke failed: expected nested shadowing output, got '$shadowing_output'" >&2
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
"${CARGO[@]}" run --bin mlg -- check examples/slices.mlg >/dev/null
"${CARGO[@]}" run --bin mlg -- build examples/slices.mlg -o target/mallang/slices >/dev/null
slices_output="$(target/mallang/slices)"
if [[ "$slices_output" != "8" ]]; then
  echo "slices native build smoke failed: expected 8, got '$slices_output'" >&2
  exit 1
fi
"${CARGO[@]}" run --bin mlg -- check examples/slice-append.mlg >/dev/null
"${CARGO[@]}" run --bin mlg -- build examples/slice-append.mlg -o target/mallang/slice-append >/dev/null
slice_append_output="$(target/mallang/slice-append)"
if [[ "$slice_append_output" != "9" ]]; then
  echo "slice append native build smoke failed: expected 9, got '$slice_append_output'" >&2
  exit 1
fi
"${CARGO[@]}" run --bin mlg -- check examples/slice-range.mlg >/dev/null
"${CARGO[@]}" run --bin mlg -- build examples/slice-range.mlg -o target/mallang/slice-range >/dev/null
slice_range_output="$(target/mallang/slice-range)"
if [[ "$slice_range_output" != "9" ]]; then
  echo "slice range native build smoke failed: expected 9, got '$slice_range_output'" >&2
  exit 1
fi
"${CARGO[@]}" run --bin mlg -- check examples/slice-element-borrow.mlg >/dev/null
"${CARGO[@]}" run --bin mlg -- build examples/slice-element-borrow.mlg -o target/mallang/slice-element-borrow >/dev/null
slice_element_borrow_output="$(target/mallang/slice-element-borrow)"
if [[ "$slice_element_borrow_output" != $'kim\nlee\n21' ]]; then
  echo "slice element borrow native build smoke failed: expected kim, lee, 21 got '$slice_element_borrow_output'" >&2
  exit 1
fi
"${CARGO[@]}" run --bin mlg -- check examples/slice-element-assignment.mlg >/dev/null
"${CARGO[@]}" run --bin mlg -- build examples/slice-element-assignment.mlg -o target/mallang/slice-element-assignment >/dev/null
slice_element_assignment_output="$(target/mallang/slice-element-assignment)"
if [[ "$slice_element_assignment_output" != $'5\npark\n40' ]]; then
  echo "slice element assignment native build smoke failed: expected 5, park, 40 got '$slice_element_assignment_output'" >&2
  exit 1
fi
"${CARGO[@]}" run --bin mlg -- check examples/indexed-field-assignment.mlg >/dev/null
"${CARGO[@]}" run --bin mlg -- build examples/indexed-field-assignment.mlg -o target/mallang/indexed-field-assignment >/dev/null
indexed_field_assignment_output="$(target/mallang/indexed-field-assignment)"
if [[ "$indexed_field_assignment_output" != $'31\npark\n21' ]]; then
  echo "indexed field assignment native build smoke failed: expected 31, park, 21 got '$indexed_field_assignment_output'" >&2
  exit 1
fi
"${CARGO[@]}" run --bin mlg -- check examples/indexed-field-read.mlg >/dev/null
"${CARGO[@]}" run --bin mlg -- build examples/indexed-field-read.mlg -o target/mallang/indexed-field-read >/dev/null
indexed_field_read_output="$(target/mallang/indexed-field-read)"
if [[ "$indexed_field_read_output" != $'User{name: kim, age: 30, profile: Profile{label: primary, score: 7}}\nlee\nprimary\n20\npark\n11' ]]; then
  echo "indexed field read native build smoke failed: expected borrowed indexed field output, got '$indexed_field_read_output'" >&2
  exit 1
fi
"${CARGO[@]}" run --bin mlg -- check examples/struct-slice-field.mlg >/dev/null
"${CARGO[@]}" run --bin mlg -- build examples/struct-slice-field.mlg -o target/mallang/struct-slice-field >/dev/null
struct_slice_field_output="$(target/mallang/struct-slice-field)"
if [[ "$struct_slice_field_output" != $'2\n1' ]]; then
  echo "struct slice field native build smoke failed: expected 2 and 1, got '$struct_slice_field_output'" >&2
  exit 1
fi
"${CARGO[@]}" run --bin mlg -- check examples/slice-field-read.mlg >/dev/null
"${CARGO[@]}" run --bin mlg -- build examples/slice-field-read.mlg -o target/mallang/slice-field-read >/dev/null
slice_field_read_output="$(target/mallang/slice-field-read)"
if [[ "$slice_field_read_output" != $'3\n2\n1\n13\n16' ]]; then
  echo "slice field read native build smoke failed: expected 3, 2, 1, 13, 16 got '$slice_field_read_output'" >&2
  exit 1
fi
"${CARGO[@]}" run --bin mlg -- check examples/slice-field-assignment.mlg >/dev/null
"${CARGO[@]}" run --bin mlg -- build examples/slice-field-assignment.mlg -o target/mallang/slice-field-assignment >/dev/null
slice_field_assignment_output="$(target/mallang/slice-field-assignment)"
if [[ "$slice_field_assignment_output" != $'5\n2\n8' ]]; then
  echo "slice field assignment native build smoke failed: expected 5, 2, 8 got '$slice_field_assignment_output'" >&2
  exit 1
fi
"${CARGO[@]}" run --bin mlg -- check examples/slice-field-append.mlg >/dev/null
"${CARGO[@]}" run --bin mlg -- build examples/slice-field-append.mlg -o target/mallang/slice-field-append >/dev/null
slice_field_append_output="$(target/mallang/slice-field-append)"
if [[ "$slice_field_append_output" != $'3\n7\n2\n9' ]]; then
  echo "slice field append native build smoke failed: expected 3, 7, 2, 9 got '$slice_field_append_output'" >&2
  exit 1
fi
"${CARGO[@]}" run --bin mlg -- check examples/indexed-slice-field-append.mlg >/dev/null
"${CARGO[@]}" run --bin mlg -- build examples/indexed-slice-field-append.mlg -o target/mallang/indexed-slice-field-append >/dev/null
indexed_slice_field_append_output="$(target/mallang/indexed-slice-field-append)"
if [[ "$indexed_slice_field_append_output" != $'3\n8\n2\n5' ]]; then
  echo "indexed slice field append native build smoke failed: expected 3, 8, 2, 5 got '$indexed_slice_field_append_output'" >&2
  exit 1
fi
"${CARGO[@]}" run --bin mlg -- check examples/slice-field-take-append.mlg >/dev/null
"${CARGO[@]}" run --bin mlg -- build examples/slice-field-take-append.mlg -o target/mallang/slice-field-take-append >/dev/null
slice_field_take_append_output="$(target/mallang/slice-field-take-append)"
if [[ "$slice_field_take_append_output" != $'3\n7\n0\n3\n8\n0' ]]; then
  echo "slice field take append native build smoke failed: expected 3, 7, 0, 3, 8, 0 got '$slice_field_take_append_output'" >&2
  exit 1
fi
"${CARGO[@]}" run --bin mlg -- check examples/slice-field-take.mlg >/dev/null
"${CARGO[@]}" run --bin mlg -- build examples/slice-field-take.mlg -o target/mallang/slice-field-take >/dev/null
slice_field_take_output="$(target/mallang/slice-field-take)"
if [[ "$slice_field_take_output" != $'2\n2\n0\n0\n0\n2\n5\n0' ]]; then
  echo "slice field take native build smoke failed: expected 2, 2, 0, 0, 0, 2, 5, 0 got '$slice_field_take_output'" >&2
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
expect_native_runtime_failure \
  "array-bounds" \
  "$runtime_fail_source" \
  "mallang runtime error: array index out of bounds"
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
expect_sanitized_native_output \
  "struct-slice-field" \
  "target/mallang/struct-slice-field.c" \
  $'2\n1'
expect_sanitized_native_output \
  "slice-field-take" \
  "target/mallang/slice-field-take.c" \
  $'2\n2\n0\n0\n0\n2\n5\n0'
expect_sanitized_native_output \
  "slice-field-take-append" \
  "target/mallang/slice-field-take-append.c" \
  $'3\n7\n0\n3\n8\n0'
expect_sanitized_native_output \
  "indexed-slice-field-append" \
  "target/mallang/indexed-slice-field-append.c" \
  $'3\n8\n2\n5'
expect_sanitized_native_output \
  "mutable-closures" \
  "target/mallang/mutable-closures.c" \
  $'1\n3\n7\n8\n9\n10\n11\n10'
expect_sanitized_native_output \
  "nested-closures" \
  "target/mallang/nested-closures.c" \
  $'17\n9\n16\n20\n17'
expect_sanitized_native_output \
  "generics" \
  "target/mallang/generics.c" \
  $'7\nmallang\n11\npair\n3\nupdated'
expect_sanitized_native_output \
  "generic-enums" \
  "target/mallang/generic-enums.c" \
  $'7\n-1\n0\n9'
expect_sanitized_native_output \
  "recursive-enums" \
  "target/mallang/recursive-enums.c" \
  $'6\n7'
expect_sanitized_native_output \
  "full-expression-cleanup" \
  "target/mallang/full-expression-cleanup.c" \
  $'42\n3\n5\n10\n0\n1\n3\n1\n2\n3\n17\n2\n1\n5\n4\n0\n1\n7\n20'
expect_sanitized_native_output \
  "string-runtime" \
  "target/mallang/string-runtime.c" \
  $'literal\nreturned\ntrue\nfield\nreplaced\n1\nindexed-after\nenum\nclosure\nmutated'
expect_sanitized_native_output \
  "standard-strings-edge" \
  "target/mallang/edge-cases.c" \
  $'0\n-1\n0\n1\n|a||b|\n9223372036854775807\n-9223372036854775808\nInvalidData\nInvalidData\nInvalidData\nInvalidData\ntrue\nfalse\nInvalidData\nError{kind: InvalidData, message: invalid integer text}\n-9223372036854775808\nfalse'
expect_sanitized_native_output \
  "borrow-range-contract" \
  "target/mallang/borrow-range-contract.c" \
  $'kim\n1\nvisited\n2\nlee\n3\nvisited\n4\nvisited\n2'
expect_all_warning_clean_generated_c
