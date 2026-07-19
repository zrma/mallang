#!/usr/bin/env bash
set -euo pipefail

if [[ $# -ne 1 ]]; then
  echo "usage: scripts/check-path-dependencies.sh <mlg-binary>" >&2
  exit 2
fi

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

MLG="$1"
CLANG_BIN="${CLANG:-clang}"
WORKSPACE="examples/projects/local-deps"
APP="$WORKSPACE/app"
MODEL="$WORKSPACE/model"
OUT_DIR="target/mallang/path-dependencies"
APP_BINARY="$OUT_DIR/pathapp"
mkdir -p "$OUT_DIR"

FORMAT_WORKSPACE="$OUT_DIR/format-workspace"
rm -rf "$FORMAT_WORKSPACE"
cp -R "$WORKSPACE" "$FORMAT_WORKSPACE"
printf 'package main\nfunc main(){print(1)}\n' >"$FORMAT_WORKSPACE/app/src/main.mlg"
printf 'package main\npub func Value()int{return 1}\n' >"$FORMAT_WORKSPACE/model/src/library.mlg"
cp "$FORMAT_WORKSPACE/model/src/library.mlg" "$OUT_DIR/dependency-format.before.mlg"
"$MLG" fmt "$FORMAT_WORKSPACE/app" >"$OUT_DIR/format.stdout" 2>"$OUT_DIR/format.stderr"
if [[ "$(cat "$OUT_DIR/format.stdout")" != "src/main.mlg: formatted" ]] || \
  [[ -s "$OUT_DIR/format.stderr" ]] || \
  ! grep -Fq 'func main() {' "$FORMAT_WORKSPACE/app/src/main.mlg" || \
  ! cmp -s "$FORMAT_WORKSPACE/model/src/library.mlg" "$OUT_DIR/dependency-format.before.mlg"; then
  echo "path dependency root-only formatting mismatch" >&2
  cat "$OUT_DIR/format.stdout" >&2
  cat "$OUT_DIR/format.stderr" >&2
  exit 1
fi

"$MLG" check "$APP" >"$OUT_DIR/app-check.stdout" 2>"$OUT_DIR/app-check.stderr"
if [[ -s "$OUT_DIR/app-check.stderr" ]]; then
  echo "path dependency app check emitted stderr" >&2
  cat "$OUT_DIR/app-check.stderr" >&2
  exit 1
fi

"$MLG" build "$APP" -o "$APP_BINARY" >"$OUT_DIR/app-build.stdout" 2>"$OUT_DIR/app-build.stderr"
if [[ "$(cat "$OUT_DIR/app-build.stdout")" != "$APP_BINARY" ]] || \
  [[ -s "$OUT_DIR/app-build.stderr" ]]; then
  echo "path dependency app build output mismatch" >&2
  cat "$OUT_DIR/app-build.stderr" >&2
  exit 1
fi
app_output="$("$APP_BINARY" 2>"$OUT_DIR/app-run.stderr")"
if [[ "$app_output" != $'42\n42' ]] || [[ -s "$OUT_DIR/app-run.stderr" ]]; then
  echo "path dependency app native output mismatch" >&2
  cat "$OUT_DIR/app-run.stderr" >&2
  exit 1
fi

"$MLG" test "$APP" >"$OUT_DIR/app-test.stdout" 2>"$OUT_DIR/app-test.stderr"
expected_app_test=$'test pathapp::DependenciesWork ... ok\ntest result: ok. 1 passed; 0 failed'
if [[ "$(cat "$OUT_DIR/app-test.stdout")" != "$expected_app_test" ]] || \
  [[ -s "$OUT_DIR/app-test.stderr" ]]; then
  echo "path dependency app test output mismatch" >&2
  cat "$OUT_DIR/app-test.stdout" >&2
  cat "$OUT_DIR/app-test.stderr" >&2
  exit 1
fi

"$MLG" check "$MODEL" >"$OUT_DIR/model-check.stdout" 2>"$OUT_DIR/model-check.stderr"
if [[ -s "$OUT_DIR/model-check.stderr" ]]; then
  echo "path dependency library check emitted stderr" >&2
  cat "$OUT_DIR/model-check.stderr" >&2
  exit 1
fi
"$MLG" test "$MODEL" >"$OUT_DIR/model-test.stdout" 2>"$OUT_DIR/model-test.stderr"
expected_model_test=$'test model::ReadsPrivateState ... ok\ntest result: ok. 1 passed; 0 failed'
if [[ "$(cat "$OUT_DIR/model-test.stdout")" != "$expected_model_test" ]] || \
  [[ -s "$OUT_DIR/model-test.stderr" ]]; then
  echo "path dependency library test output mismatch" >&2
  cat "$OUT_DIR/model-test.stdout" >&2
  cat "$OUT_DIR/model-test.stderr" >&2
  exit 1
fi

for command in build run; do
  stdout_path="$OUT_DIR/model-$command.stdout"
  stderr_path="$OUT_DIR/model-$command.stderr"
  if "$MLG" "$command" "$MODEL" >"$stdout_path" 2>"$stderr_path"; then
    echo "path dependency library $command unexpectedly succeeded" >&2
    exit 1
  fi
  if [[ -s "$stdout_path" ]] || ! grep -Fq 'src/main.mlg: project entry source is missing' "$stderr_path"; then
    echo "path dependency library $command diagnostic mismatch" >&2
    cat "$stderr_path" >&2
    exit 1
  fi
done

check_generated_program() {
  local label="$1"
  local c_source="$2"
  local expected_stdout="$3"
  local runner_case="${4:-}"
  local c_source_abs
  local accounting_source="$OUT_DIR/$label-accounting.c"
  local strict_binary="$OUT_DIR/$label-strict"
  local sanitizer_binary="$OUT_DIR/$label-sanitized"
  local stderr_path="$OUT_DIR/$label.stderr"
  local main_setup=""
  local main_call="mallang_program_main()"
  local output

  c_source_abs="$(cd "$(dirname "$c_source")" && pwd)/$(basename "$c_source")"
  if [[ -n "$runner_case" ]]; then
    main_setup=$'    char mlg_program[] = "mallang-test";\n    char mlg_case[] = "'"$runner_case"$'";\n    char *mlg_argv[] = {mlg_program, mlg_case, NULL};'
    main_call="mallang_program_main(2, mlg_argv)"
  fi
  cat >"$accounting_source" <<EOF
#define main mallang_program_main
#include "$c_source_abs"
#undef main

int main(void) {
    if (mallang_live_allocation_count() != 0) {
        return 10;
    }
$main_setup
    int status = $main_call;
    if (status != 0 || mallang_live_allocation_count() != 0) {
        return 11;
    }
    return 0;
}
EOF

  "$CLANG_BIN" -std=c11 -Wall -Wextra -Werror -pedantic \
    "$accounting_source" -o "$strict_binary" 2>"$stderr_path"
  if [[ -s "$stderr_path" ]]; then
    echo "path dependency strict C compile emitted stderr for $label" >&2
    cat "$stderr_path" >&2
    exit 1
  fi
  output="$("$strict_binary" 2>"$stderr_path")"
  if [[ "$output" != "$expected_stdout" ]] || [[ -s "$stderr_path" ]]; then
    echo "path dependency strict C run mismatch for $label" >&2
    cat "$stderr_path" >&2
    exit 1
  fi

  "$CLANG_BIN" -std=c11 -Wall -Wextra -Werror -pedantic \
    -fsanitize=address,undefined -fno-omit-frame-pointer \
    "$accounting_source" -o "$sanitizer_binary" 2>"$stderr_path"
  if [[ -s "$stderr_path" ]]; then
    echo "path dependency sanitizer compile emitted stderr for $label" >&2
    cat "$stderr_path" >&2
    exit 1
  fi
  output="$("$sanitizer_binary" 2>"$stderr_path")"
  if [[ "$output" != "$expected_stdout" ]] || [[ -s "$stderr_path" ]]; then
    echo "path dependency sanitizer run mismatch for $label" >&2
    cat "$stderr_path" >&2
    exit 1
  fi
}

check_generated_program "app" "$APP/target/mallang/pathapp.c" $'42\n42'
check_generated_program "app-test" "$APP/target/mallang/tests/runner.c" "" "0"
check_generated_program "model-test" "$MODEL/target/mallang/tests/runner.c" "" "0"

echo "local path dependency graph, library workflow, native accounting, strict C, and sanitizer smoke passed"
