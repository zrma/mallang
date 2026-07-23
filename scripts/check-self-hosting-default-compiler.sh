#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

if [[ $# -ne 0 ]]; then
  echo "usage: scripts/check-self-hosting-default-compiler.sh" >&2
  exit 2
fi

if command -v cargo >/dev/null 2>&1; then
  cargo_command=(cargo)
elif command -v rustup >/dev/null 2>&1; then
  cargo_command=(rustup run stable cargo)
else
  echo "default compiler transition check failed: cargo is required" >&2
  exit 1
fi
command -v clang >/dev/null 2>&1 || {
  echo "default compiler transition check failed: clang is required" >&2
  exit 1
}

work="target/mallang/self-hosting/b5-default"
driver="target/debug/mlg"
self_compiler="target/debug/mlgc"
fixture="bootstrap/compiler/fixtures/backend/scalars.mlg"
ir_fixtures="bootstrap/compiler/fixtures/ir"
semantic_rejection="bootstrap/compiler/fixtures/semantic/body-unknown-variable.mlg"
parser_rejection="bootstrap/compiler/fixtures/parser/recovery-statements.mlg"
project_fixture="examples/projects/local-deps/app"
project_artifact="pathapp"
project_rejection="tests/fixtures/diagnostics/dependency-project/app"
test_fixture="examples/projects/hello"
mkdir -p "$work"

invalid_dependency="$work/invalid-dependency"
dependency_cycle="$work/dependency-cycle"
rm -rf "$invalid_dependency" "$dependency_cycle"
mkdir -p \
  "$invalid_dependency/src" \
  "$dependency_cycle/src" \
  "$dependency_cycle/deps/text/src"
printf '%s\n' \
  '[project]' \
  'name = "app"' \
  '' \
  '[dependencies]' \
  'text = { path = "/tmp/text" }' \
  >"$invalid_dependency/mallang.toml"
printf '%s\n' 'func main() {}' >"$invalid_dependency/src/main.mlg"
printf '%s\n' \
  '[project]' \
  'name = "app"' \
  '' \
  '[dependencies]' \
  'text = { path = "deps/text" }' \
  >"$dependency_cycle/mallang.toml"
printf '%s\n' 'func main() {}' >"$dependency_cycle/src/main.mlg"
printf '%s\n' \
  '[project]' \
  'name = "text"' \
  '' \
  '[dependencies]' \
  'app = { path = "../.." }' \
  >"$dependency_cycle/deps/text/mallang.toml"
printf '%s\n' 'package main' >"$dependency_cycle/deps/text/src/text.mlg"

"${cargo_command[@]}" build --locked --quiet --lib --bin mlg
scripts/build-self-hosted-compiler.sh --stage0 "$driver" --output "$self_compiler" >/dev/null

scripts/check-formatter.sh "$driver" --compiler stage0 >/dev/null
scripts/check-formatter.sh \
  "$driver" --compiler self --self-compiler "$self_compiler" >/dev/null
scripts/check-test-workflow.sh "$driver" --compiler stage0 >/dev/null
scripts/check-test-workflow.sh \
  "$driver" --compiler self --self-compiler "$self_compiler" >/dev/null
"$driver" --compiler self --self-compiler "$self_compiler" \
  build examples/process-io.mlg -o "$work/process-io" >/dev/null
scripts/check-process-io-runtime.sh \
  "$driver" --compiler self --self-compiler "$self_compiler" >/dev/null

formatter_corpus="$work/formatter-corpus"
rm -rf "$formatter_corpus"
formatter_fixture_count=0
while IFS= read -r source_path; do
  stage0_path="$formatter_corpus/stage0/$source_path"
  self_path="$formatter_corpus/self/$source_path"
  mkdir -p "$(dirname "$stage0_path")" "$(dirname "$self_path")"
  cp "$source_path" "$stage0_path"
  cp "$source_path" "$self_path"
  "$driver" --compiler stage0 fmt "$stage0_path" >/dev/null
  "$driver" --compiler self --self-compiler "$self_compiler" \
    fmt "$self_path" >/dev/null
  if ! cmp -s "$stage0_path" "$self_path"; then
    echo "public Stage0/self formatter corpus parity failed: $source_path" >&2
    diff -u "$stage0_path" "$self_path" >&2 || true
    exit 1
  fi
  formatter_fixture_count=$((formatter_fixture_count + 1))
done < <(
  find examples bootstrap/compiler/src -type f -name '*.mlg' -print |
    LC_ALL=C sort
)
if [[ "$formatter_fixture_count" -lt 20 ]]; then
  echo "public formatter corpus unexpectedly small: $formatter_fixture_count" >&2
  exit 1
fi

crate_version="$(
  sed -n '/^\[package\]/,/^\[/ s/^version = "\([^"]*\)"/\1/p' Cargo.toml | head -n 1
)"
if [[ "$("$driver" --version)" != "mlg $crate_version" ]] || \
   [[ "$("$driver" --compiler stage0 --version)" != "mlg $crate_version" ]] || \
   [[ "$("$driver" --compiler self --version)" != "mlg $crate_version" ]] || \
   [[ "$("$driver" --compiler self --self-compiler "$self_compiler" --version)" != "mlg $crate_version" ]]; then
  echo "default compiler transition version compatibility failed" >&2
  exit 1
fi

stage0_provenance="$work/stage0.provenance"
self_provenance="$work/self.provenance"
"$driver" --compiler stage0 --version --verbose >"$stage0_provenance"
"$driver" --compiler self --version --verbose >"$self_provenance"
if [[ "$(cat "$stage0_provenance")" != $'mlg '"$crate_version"$'\ndriver: rust\ncompiler: stage0\ncore: rust-stage0' ]]; then
  echo "Stage0 compiler provenance mismatch" >&2
  cat "$stage0_provenance" >&2
  exit 1
fi
if [[ "$(cat "$self_provenance")" != $'mlg '"$crate_version"$'\ndriver: rust\ncompiler: self\ncore: mlgc protocol 1' ]]; then
  echo "self-hosted compiler provenance mismatch" >&2
  cat "$self_provenance" >&2
  exit 1
fi

stage0_binary="$work/scalars.stage0"
self_binary="$work/scalars.self"
"$driver" --compiler stage0 build "$fixture" -o "$stage0_binary" \
  >"$work/build.stage0.stdout" 2>"$work/build.stage0.stderr"
cp target/mallang/scalars.c "$work/scalars.stage0.c"
"$driver" --compiler self build "$fixture" -o "$self_binary" \
  >"$work/build.self.stdout" 2>"$work/build.self.stderr"
cp target/mallang/scalars.c "$work/scalars.self.c"
if [[ "$(cat "$work/build.stage0.stdout")" != "$stage0_binary" ]] || \
   [[ "$(cat "$work/build.self.stdout")" != "$self_binary" ]] || \
   [[ -s "$work/build.stage0.stderr" || -s "$work/build.self.stderr" ]] || \
   ! cmp -s "$work/scalars.stage0.c" "$work/scalars.self.c"; then
  echo "public Stage0/self build parity failed" >&2
  exit 1
fi

set +e
"$driver" --compiler stage0 run "$fixture" \
  >"$work/run.stage0.stdout" 2>"$work/run.stage0.stderr"
stage0_status=$?
"$driver" --compiler self run "$fixture" \
  >"$work/run.self.stdout" 2>"$work/run.self.stderr"
self_status=$?
set -e
if [[ "$stage0_status" -ne 0 || "$self_status" -ne 0 ]] || \
   [[ "$stage0_status" -ne "$self_status" ]] || \
   ! cmp -s "$work/run.stage0.stdout" "$work/run.self.stdout" || \
   ! cmp -s "$work/run.stage0.stderr" "$work/run.self.stderr"; then
  echo "public Stage0/self run parity failed" >&2
  exit 1
fi

"$driver" --compiler stage0 check "$project_fixture" \
  >"$work/project-check.stage0.stdout" 2>"$work/project-check.stage0.stderr"
"$driver" --compiler self check "$project_fixture" \
  >"$work/project-check.self.stdout" 2>"$work/project-check.self.stderr"
if ! cmp -s "$work/project-check.stage0.stdout" "$work/project-check.self.stdout" || \
   ! cmp -s "$work/project-check.stage0.stderr" "$work/project-check.self.stderr"; then
  echo "public Stage0/self project check parity failed" >&2
  exit 1
fi

ir_fixture_count=0
for ir_fixture in "$ir_fixtures"/*.mlg; do
  ir_name="$(basename "$ir_fixture" .mlg)"
  "$driver" --compiler stage0 ir "$ir_fixture" \
    >"$work/ir-$ir_name.stage0.stdout" 2>"$work/ir-$ir_name.stage0.stderr"
  "$driver" --compiler self ir "$ir_fixture" \
    >"$work/ir-$ir_name.self.stdout" 2>"$work/ir-$ir_name.self.stderr"
  if ! cmp -s "$work/ir-$ir_name.stage0.stdout" "$work/ir-$ir_name.self.stdout" || \
     ! cmp -s "$work/ir-$ir_name.stage0.stderr" "$work/ir-$ir_name.self.stderr"; then
    echo "public Stage0/self IR corpus parity failed: $ir_fixture" >&2
    exit 1
  fi
  ir_fixture_count=$((ir_fixture_count + 1))
done
if [[ "$ir_fixture_count" -lt 48 ]]; then
  echo "public IR corpus unexpectedly small: $ir_fixture_count" >&2
  exit 1
fi

"$driver" --compiler stage0 ir "$project_fixture" \
  >"$work/project-ir.stage0.stdout" 2>"$work/project-ir.stage0.stderr"
"$driver" --compiler self ir "$project_fixture" \
  >"$work/project-ir.self.stdout" 2>"$work/project-ir.self.stderr"
if ! cmp -s "$work/project-ir.stage0.stdout" "$work/project-ir.self.stdout" || \
   ! cmp -s "$work/project-ir.stage0.stderr" "$work/project-ir.self.stderr"; then
  echo "public Stage0/self project IR parity failed" >&2
  exit 1
fi

spy_compiler="$work/mlgc-spy"
spy_log="$work/mlgc-spy.log"
printf '%s\n' \
  '#!/usr/bin/env bash' \
  'set -euo pipefail' \
  'printf "%s\n" "${1:-}" >>"$MLG_SPY_LOG"' \
  'exec "$MLG_SPY_TARGET" "$@"' \
  >"$spy_compiler"
chmod +x "$spy_compiler"
: >"$spy_log"
MLG_SPY_LOG="$spy_log" \
  MLG_SPY_TARGET="$ROOT/$self_compiler" \
  "$driver" --compiler self --self-compiler "$spy_compiler" check "$project_fixture" \
  >"$work/project-spy.stdout" 2>"$work/project-spy.stderr"
if [[ "$(grep -c '^manifest$' "$spy_log")" -ne 3 ]] || \
   [[ "$(grep -c '^project-plan$' "$spy_log")" -ne 1 ]] || \
   [[ "$(grep -c '^check-project$' "$spy_log")" -ne 1 ]] || \
   [[ -s "$work/project-spy.stderr" ]]; then
  echo "public self-hosted project protocol routing mismatch" >&2
  cat "$spy_log" >&2
  exit 1
fi

: >"$spy_log"
MLG_SPY_LOG="$spy_log" \
  MLG_SPY_TARGET="$ROOT/$self_compiler" \
  "$driver" --compiler self --self-compiler "$spy_compiler" ir "$project_fixture" \
  >"$work/project-ir-spy.stdout" 2>"$work/project-ir-spy.stderr"
if [[ "$(grep -c '^manifest$' "$spy_log")" -ne 3 ]] || \
   [[ "$(grep -c '^project-plan$' "$spy_log")" -ne 1 ]] || \
   [[ "$(grep -c '^ir-project$' "$spy_log")" -ne 1 ]] || \
   [[ -s "$work/project-ir-spy.stderr" ]]; then
  echo "public self-hosted project IR protocol routing mismatch" >&2
  cat "$spy_log" >&2
  exit 1
fi

: >"$spy_log"
MLG_SPY_LOG="$spy_log" \
  MLG_SPY_TARGET="$ROOT/$self_compiler" \
  "$driver" --compiler self --self-compiler "$spy_compiler" \
  fmt --check examples/hello.mlg \
  >"$work/formatter-spy.stdout" 2>"$work/formatter-spy.stderr"
if [[ "$(grep -c '^format$' "$spy_log")" -ne 1 ]] || \
   [[ "$(wc -l <"$spy_log")" -ne 1 ]] || \
   [[ -s "$work/formatter-spy.stdout" || -s "$work/formatter-spy.stderr" ]]; then
  echo "public self-hosted formatter protocol routing mismatch" >&2
  cat "$spy_log" >&2
  exit 1
fi

: >"$spy_log"
MLG_SPY_LOG="$spy_log" \
  MLG_SPY_TARGET="$ROOT/$self_compiler" \
  "$driver" --compiler self --self-compiler "$spy_compiler" \
  test "$test_fixture" --exact "hello::GenericAndClosure" \
  >"$work/test-spy.stdout" 2>"$work/test-spy.stderr"
if [[ "$(grep -c '^manifest$' "$spy_log")" -ne 1 ]] || \
   [[ "$(grep -c '^project-plan$' "$spy_log")" -ne 1 ]] || \
   [[ "$(grep -c '^test-project$' "$spy_log")" -ne 1 ]] || \
   [[ "$(wc -l <"$spy_log")" -ne 3 ]] || \
   [[ -s "$work/test-spy.stderr" ]]; then
  echo "public self-hosted test protocol routing mismatch" >&2
  cat "$spy_log" >&2
  exit 1
fi

: >"$spy_log"
MLG_SPY_LOG="$spy_log" \
  MLG_SPY_TARGET="$ROOT/$self_compiler" \
  "$driver" --compiler self --self-compiler "$spy_compiler" \
  build "$fixture" -o "$work/native-spy-build" \
  >"$work/native-build-spy.stdout" 2>"$work/native-build-spy.stderr"
if [[ "$(grep -c '^native-build$' "$spy_log")" -ne 1 ]] || \
   [[ "$(wc -l <"$spy_log")" -ne 1 ]] || \
   [[ -s "$work/native-build-spy.stderr" ]]; then
  echo "public self-hosted native build protocol routing mismatch" >&2
  cat "$spy_log" >&2
  exit 1
fi

: >"$spy_log"
MLG_SPY_LOG="$spy_log" \
  MLG_SPY_TARGET="$ROOT/$self_compiler" \
  "$driver" --compiler self --self-compiler "$spy_compiler" \
  run "$fixture" \
  >"$work/native-run-spy.stdout" 2>"$work/native-run-spy.stderr"
if [[ "$(grep -c '^native-run$' "$spy_log")" -ne 1 ]] || \
   [[ "$(wc -l <"$spy_log")" -ne 1 ]] || \
   [[ -s "$work/native-run-spy.stderr" ]]; then
  echo "public self-hosted native run protocol routing mismatch" >&2
  cat "$spy_log" >&2
  exit 1
fi

: >"$spy_log"
MLG_SPY_LOG="$spy_log" \
  MLG_SPY_TARGET="$ROOT/$self_compiler" \
  "$driver" --compiler self --self-compiler "$spy_compiler" \
  build "$project_fixture" -o "$work/native-project-spy-build" \
  >"$work/native-project-build-spy.stdout" 2>"$work/native-project-build-spy.stderr"
if [[ "$(grep -c '^manifest$' "$spy_log")" -ne 3 ]] || \
   [[ "$(grep -c '^project-plan$' "$spy_log")" -ne 1 ]] || \
   [[ "$(grep -c '^native-build-project$' "$spy_log")" -ne 1 ]] || \
   [[ "$(wc -l <"$spy_log")" -ne 5 ]] || \
   [[ -s "$work/native-project-build-spy.stderr" ]]; then
  echo "public self-hosted native project build protocol routing mismatch" >&2
  cat "$spy_log" >&2
  exit 1
fi

: >"$spy_log"
MLG_SPY_LOG="$spy_log" \
  MLG_SPY_TARGET="$ROOT/$self_compiler" \
  "$driver" --compiler self --self-compiler "$spy_compiler" \
  run "$project_fixture" \
  >"$work/native-project-run-spy.stdout" 2>"$work/native-project-run-spy.stderr"
if [[ "$(grep -c '^manifest$' "$spy_log")" -ne 3 ]] || \
   [[ "$(grep -c '^project-plan$' "$spy_log")" -ne 1 ]] || \
   [[ "$(grep -c '^native-run-project$' "$spy_log")" -ne 1 ]] || \
   [[ "$(wc -l <"$spy_log")" -ne 5 ]] || \
   [[ -s "$work/native-project-run-spy.stderr" ]]; then
  echo "public self-hosted native project run protocol routing mismatch" >&2
  cat "$spy_log" >&2
  exit 1
fi

formatter_json="$work/formatter-json.mlg"
printf '%s\n' 'func main(){print(1)}' >"$formatter_json"
set +e
"$driver" --diagnostic-format json --compiler stage0 \
  fmt --check "$formatter_json" \
  >"$work/formatter-json.stage0.stdout" 2>"$work/formatter-json.stage0.stderr"
stage0_status=$?
"$driver" --diagnostic-format json --compiler self \
  fmt --check "$formatter_json" \
  >"$work/formatter-json.self.stdout" 2>"$work/formatter-json.self.stderr"
self_status=$?
set -e
if [[ "$stage0_status" -eq 0 || "$self_status" -eq 0 ]] || \
   [[ "$stage0_status" -ne "$self_status" ]] || \
   ! cmp -s "$work/formatter-json.stage0.stdout" "$work/formatter-json.self.stdout" || \
   ! cmp -s "$work/formatter-json.stage0.stderr" "$work/formatter-json.self.stderr"; then
  echo "public Stage0/self JSON formatter check parity failed" >&2
  exit 1
fi

for diagnostic_format in human json; do
  name="formatter-rejection.$diagnostic_format"
  set +e
  if [[ "$diagnostic_format" == "json" ]]; then
    "$driver" --diagnostic-format json --compiler stage0 fmt "$parser_rejection" \
      >"$work/$name.stage0.stdout" 2>"$work/$name.stage0.stderr"
    stage0_status=$?
    "$driver" --diagnostic-format json --compiler self fmt "$parser_rejection" \
      >"$work/$name.self.stdout" 2>"$work/$name.self.stderr"
    self_status=$?
  else
    "$driver" --compiler stage0 fmt "$parser_rejection" \
      >"$work/$name.stage0.stdout" 2>"$work/$name.stage0.stderr"
    stage0_status=$?
    "$driver" --compiler self fmt "$parser_rejection" \
      >"$work/$name.self.stdout" 2>"$work/$name.self.stderr"
    self_status=$?
  fi
  set -e
  if [[ "$stage0_status" -eq 0 || "$self_status" -eq 0 ]] || \
     [[ "$stage0_status" -ne "$self_status" ]] || \
     ! cmp -s "$work/$name.stage0.stdout" "$work/$name.self.stdout" || \
     ! cmp -s "$work/$name.stage0.stderr" "$work/$name.self.stderr"; then
    echo "public Stage0/self $diagnostic_format formatter rejection parity failed" >&2
    exit 1
  fi
done

malformed_formatter="$work/mlgc-malformed-formatter"
printf '%s\n' \
  '#!/usr/bin/env bash' \
  'set -euo pipefail' \
  'printf "FORMAT|1|2\n\n"' \
  >"$malformed_formatter"
chmod +x "$malformed_formatter"
cp examples/hello.mlg "$work/malformed-format.mlg"
cp "$work/malformed-format.mlg" "$work/malformed-format.before.mlg"
if "$driver" --compiler self --self-compiler "$malformed_formatter" \
  fmt "$work/malformed-format.mlg" \
  >"$work/malformed-format.stdout" 2>"$work/malformed-format.stderr"; then
  echo "malformed formatter response unexpectedly succeeded" >&2
  exit 1
fi
if [[ -s "$work/malformed-format.stdout" ]] || \
   ! grep -Fq 'format payload byte length does not match its header' \
     "$work/malformed-format.stderr" || \
   ! cmp -s "$work/malformed-format.mlg" "$work/malformed-format.before.mlg"; then
  echo "malformed formatter response handling mismatch" >&2
  exit 1
fi

malformed_test="$work/mlgc-malformed-test"
printf '%s\n' \
  '#!/usr/bin/env bash' \
  'set -euo pipefail' \
  'if [[ "${1:-}" == "test-project" ]]; then' \
  '  printf "TEST|1|1|2\nCASE|0|0|0|98,97,100\n\n"' \
  '  exit 0' \
  'fi' \
  'exec "$MLG_MALFORMED_TARGET" "$@"' \
  >"$malformed_test"
chmod +x "$malformed_test"
cp "$test_fixture/target/mallang/tests/runner.c" "$work/malformed-test.before.c"
if MLG_MALFORMED_TARGET="$ROOT/$self_compiler" \
  "$driver" --compiler self --self-compiler "$malformed_test" \
  test "$test_fixture" \
  >"$work/malformed-test.stdout" 2>"$work/malformed-test.stderr"; then
  echo "malformed test response unexpectedly succeeded" >&2
  exit 1
fi
if [[ -s "$work/malformed-test.stdout" ]] || \
   ! grep -Fq 'test C payload byte length does not match its header' \
     "$work/malformed-test.stderr" || \
   ! cmp -s "$test_fixture/target/mallang/tests/runner.c" \
     "$work/malformed-test.before.c"; then
  echo "malformed test response handling mismatch" >&2
  exit 1
fi

malformed_native="$work/mlgc-malformed-native"
printf '%s\n' \
  '#!/usr/bin/env bash' \
  'set -euo pipefail' \
  'if [[ "${1:-}" == "native-build" ]]; then' \
  '  printf "NATIVE|1|build|2\n\n"' \
  '  exit 0' \
  'fi' \
  'exec "$MLG_MALFORMED_TARGET" "$@"' \
  >"$malformed_native"
chmod +x "$malformed_native"
malformed_native_output="$work/malformed-native-output"
printf '%s\n' 'preserved-native-output' >"$malformed_native_output"
cp target/mallang/scalars.c "$work/malformed-native.before.c"
if MLG_MALFORMED_TARGET="$ROOT/$self_compiler" \
  "$driver" --compiler self --self-compiler "$malformed_native" \
  build "$fixture" -o "$malformed_native_output" \
  >"$work/malformed-native.stdout" 2>"$work/malformed-native.stderr"; then
  echo "malformed native response unexpectedly succeeded" >&2
  exit 1
fi
if [[ -s "$work/malformed-native.stdout" ]] || \
   ! grep -Fq 'native C payload byte length does not match its header' \
     "$work/malformed-native.stderr" || \
   [[ "$(cat "$malformed_native_output")" != "preserved-native-output" ]] || \
   ! cmp -s target/mallang/scalars.c "$work/malformed-native.before.c"; then
  echo "malformed native response handling mismatch" >&2
  exit 1
fi

"$driver" --compiler stage0 build "$project_fixture" -o "$work/project.stage0" \
  >"$work/project-build.stage0.stdout" 2>"$work/project-build.stage0.stderr"
cp "$project_fixture/target/mallang/$project_artifact.c" "$work/project.stage0.c"
"$driver" --compiler self build "$project_fixture" -o "$work/project.self" \
  >"$work/project-build.self.stdout" 2>"$work/project-build.self.stderr"
cp "$project_fixture/target/mallang/$project_artifact.c" "$work/project.self.c"
if [[ -s "$work/project-build.stage0.stderr" || -s "$work/project-build.self.stderr" ]] || \
   ! cmp -s "$work/project.stage0.c" "$work/project.self.c"; then
  echo "public Stage0/self project build parity failed" >&2
  exit 1
fi

set +e
"$driver" --compiler stage0 run "$project_fixture" \
  >"$work/project-run.stage0.stdout" 2>"$work/project-run.stage0.stderr"
stage0_status=$?
"$driver" --compiler self run "$project_fixture" \
  >"$work/project-run.self.stdout" 2>"$work/project-run.self.stderr"
self_status=$?
set -e
if [[ "$stage0_status" -ne 0 || "$self_status" -ne 0 ]] || \
   [[ "$stage0_status" -ne "$self_status" ]] || \
   ! cmp -s "$work/project-run.stage0.stdout" "$work/project-run.self.stdout" || \
   ! cmp -s "$work/project-run.stage0.stderr" "$work/project-run.self.stderr"; then
  echo "public Stage0/self project run parity failed" >&2
  exit 1
fi

"$driver" --compiler stage0 check "$fixture" \
  >"$work/check.stage0.stdout" 2>"$work/check.stage0.stderr"
"$driver" --compiler self check "$fixture" \
  >"$work/check.self.stdout" 2>"$work/check.self.stderr"
if ! cmp -s "$work/check.stage0.stdout" "$work/check.self.stdout" || \
   ! cmp -s "$work/check.stage0.stderr" "$work/check.self.stderr"; then
  echo "public Stage0/self check success parity failed" >&2
  exit 1
fi

for diagnostic_format in human json; do
  for rejection in "$semantic_rejection" "$parser_rejection"; do
    name="$(basename "$rejection" .mlg).$diagnostic_format"
    set +e
    if [[ "$diagnostic_format" == "json" ]]; then
      "$driver" --diagnostic-format json --compiler stage0 check "$rejection" \
        >"$work/$name.stage0.stdout" 2>"$work/$name.stage0.stderr"
    else
      "$driver" --compiler stage0 check "$rejection" \
        >"$work/$name.stage0.stdout" 2>"$work/$name.stage0.stderr"
    fi
    stage0_status=$?
    if [[ "$diagnostic_format" == "json" ]]; then
      "$driver" --diagnostic-format json --compiler self check "$rejection" \
        >"$work/$name.self.stdout" 2>"$work/$name.self.stderr"
    else
      "$driver" --compiler self check "$rejection" \
        >"$work/$name.self.stdout" 2>"$work/$name.self.stderr"
    fi
    self_status=$?
    set -e
    if [[ "$stage0_status" -eq 0 || "$self_status" -eq 0 ]] || \
       [[ "$stage0_status" -ne "$self_status" ]] || \
       ! cmp -s "$work/$name.stage0.stdout" "$work/$name.self.stdout" || \
       ! cmp -s "$work/$name.stage0.stderr" "$work/$name.self.stderr"; then
      echo "public Stage0/self $diagnostic_format rejection parity failed: $rejection" >&2
      exit 1
    fi
  done
done

for diagnostic_format in human json; do
  name="project-rejection.$diagnostic_format"
  set +e
  if [[ "$diagnostic_format" == "json" ]]; then
    "$driver" --diagnostic-format json --compiler stage0 check "$project_rejection" \
      >"$work/$name.stage0.stdout" 2>"$work/$name.stage0.stderr"
  else
    "$driver" --compiler stage0 check "$project_rejection" \
      >"$work/$name.stage0.stdout" 2>"$work/$name.stage0.stderr"
  fi
  stage0_status=$?
  if [[ "$diagnostic_format" == "json" ]]; then
    "$driver" --diagnostic-format json --compiler self check "$project_rejection" \
      >"$work/$name.self.stdout" 2>"$work/$name.self.stderr"
  else
    "$driver" --compiler self check "$project_rejection" \
      >"$work/$name.self.stdout" 2>"$work/$name.self.stderr"
  fi
  self_status=$?
  set -e
  if [[ "$stage0_status" -eq 0 || "$self_status" -eq 0 ]] || \
     [[ "$stage0_status" -ne "$self_status" ]] || \
     ! cmp -s "$work/$name.stage0.stdout" "$work/$name.self.stdout" || \
     ! cmp -s "$work/$name.stage0.stderr" "$work/$name.self.stderr"; then
    echo "public Stage0/self $diagnostic_format project rejection parity failed" >&2
    exit 1
  fi
done

for diagnostic_format in human json; do
  for ir_rejection in "$semantic_rejection" "$project_rejection"; do
    name="ir-$(basename "$ir_rejection" .mlg).$diagnostic_format"
    set +e
    if [[ "$diagnostic_format" == "json" ]]; then
      "$driver" --diagnostic-format json --compiler stage0 ir "$ir_rejection" \
        >"$work/$name.stage0.stdout" 2>"$work/$name.stage0.stderr"
      stage0_status=$?
      "$driver" --diagnostic-format json --compiler self ir "$ir_rejection" \
        >"$work/$name.self.stdout" 2>"$work/$name.self.stderr"
      self_status=$?
    else
      "$driver" --compiler stage0 ir "$ir_rejection" \
        >"$work/$name.stage0.stdout" 2>"$work/$name.stage0.stderr"
      stage0_status=$?
      "$driver" --compiler self ir "$ir_rejection" \
        >"$work/$name.self.stdout" 2>"$work/$name.self.stderr"
      self_status=$?
    fi
    set -e
    if [[ "$stage0_status" -eq 0 || "$self_status" -eq 0 ]] || \
       [[ "$stage0_status" -ne "$self_status" ]] || \
       ! cmp -s "$work/$name.stage0.stdout" "$work/$name.self.stdout" || \
       ! cmp -s "$work/$name.stage0.stderr" "$work/$name.self.stderr"; then
      echo "public Stage0/self $diagnostic_format IR rejection parity failed: $ir_rejection" >&2
      exit 1
    fi
  done
done

for graph_rejection in "$invalid_dependency" "$dependency_cycle"; do
  rejection_name="$(basename "$graph_rejection")"
  for diagnostic_format in human json; do
    name="$rejection_name.$diagnostic_format"
    set +e
    if [[ "$diagnostic_format" == "json" ]]; then
      "$driver" --diagnostic-format json --compiler stage0 check "$graph_rejection" \
        >"$work/$name.stage0.stdout" 2>"$work/$name.stage0.stderr"
      stage0_status=$?
      "$driver" --diagnostic-format json --compiler self check "$graph_rejection" \
        >"$work/$name.self.stdout" 2>"$work/$name.self.stderr"
      self_status=$?
    else
      "$driver" --compiler stage0 check "$graph_rejection" \
        >"$work/$name.stage0.stdout" 2>"$work/$name.stage0.stderr"
      stage0_status=$?
      "$driver" --compiler self check "$graph_rejection" \
        >"$work/$name.self.stdout" 2>"$work/$name.self.stderr"
      self_status=$?
    fi
    set -e
    if [[ "$stage0_status" -eq 0 || "$self_status" -eq 0 ]] || \
       [[ "$stage0_status" -ne "$self_status" ]] || \
       ! cmp -s "$work/$name.stage0.stdout" "$work/$name.self.stdout" || \
       ! cmp -s "$work/$name.stage0.stderr" "$work/$name.self.stderr"; then
      echo "public Stage0/self $diagnostic_format project graph rejection parity failed: $graph_rejection" >&2
      exit 1
    fi
  done
done

if "$driver" --compiler self --self-compiler "$work/missing-mlgc" \
  build "$fixture" -o "$work/missing" \
  >"$work/missing.stdout" 2>"$work/missing.stderr"; then
  echo "missing self-hosted compiler unexpectedly fell back to Stage0" >&2
  exit 1
fi
if [[ -s "$work/missing.stdout" ]] || \
   ! grep -Fq 'self-hosted compiler not found' "$work/missing.stderr"; then
  echo "missing self-hosted compiler diagnostic mismatch" >&2
  exit 1
fi

echo "B5 default compiler transition gate passed: core=mlgc protocol=1 inputs=standalone,project commands=check,fmt,ir,build,run,test diagnostics=human,json fallback=explicit-only"
