#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

CARGO=(cargo)
if [[ -n "${CARGO_BIN:-}" ]]; then
  CARGO=("$CARGO_BIN")
fi

RELEASE_BIN="target/release/mlg"
SMOKE_BIN="target/mallang/release-binary-first"
NEGATIVE_DIR="target/mallang/release-binary-negative"

mkdir -p target/mallang "$NEGATIVE_DIR"

expect_release_check_failure() {
  local label="$1"
  local expected_stderr="$2"
  local source="$NEGATIVE_DIR/$label.mlg"
  local stdout="$NEGATIVE_DIR/$label.stdout"
  local stderr="$NEGATIVE_DIR/$label.stderr"

  if "$RELEASE_BIN" check "$source" >"$stdout" 2>"$stderr"; then
    echo "release binary $label failure smoke failed: expected non-zero exit" >&2
    exit 1
  fi

  if [[ -s "$stdout" ]]; then
    echo "release binary $label failure smoke failed: expected empty stdout" >&2
    cat "$stdout" >&2
    exit 1
  fi

  if ! grep -Fq "$expected_stderr" "$stderr"; then
    echo "release binary $label failure smoke failed: expected stderr containing '$expected_stderr'" >&2
    echo "stderr was:" >&2
    cat "$stderr" >&2
    exit 1
  fi
}

"${CARGO[@]}" build --release --bin mlg

crate_version="$(sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml)"
version_output="$("$RELEASE_BIN" --version)"
if [[ "$version_output" != "mlg $crate_version" ]]; then
  echo "release binary version smoke failed: expected mlg $crate_version, got '$version_output'" >&2
  exit 1
fi

help_output="$("$RELEASE_BIN" --help)"
if [[ "$help_output" != *"usage:"* || "$help_output" != *"$RELEASE_BIN check <source-file>"* || "$help_output" != *"$RELEASE_BIN --version"* ]]; then
  echo "release binary help smoke failed" >&2
  echo "$help_output" >&2
  exit 1
fi

lex_output="$("$RELEASE_BIN" lex examples/first.mlg)"
if [[ "$lex_output" != *"Keyword(Func) @ 0..4"* || "$lex_output" != *'Ident("add")'* ]]; then
  echo "release binary lex smoke failed" >&2
  echo "$lex_output" >&2
  exit 1
fi

parse_output="$("$RELEASE_BIN" parse examples/first.mlg)"
if [[ "$parse_output" != *"Program {"* || "$parse_output" != *'name: "main"'* || "$parse_output" != *"Function {"* ]]; then
  echo "release binary parse smoke failed" >&2
  echo "$parse_output" >&2
  exit 1
fi

check_output="$("$RELEASE_BIN" check examples/first.mlg)"
if [[ "$check_output" != "examples/first.mlg: ok" ]]; then
  echo "release binary check smoke failed: $check_output" >&2
  exit 1
fi

cat >"$NEGATIVE_DIR/use-after-move.mlg" <<'MLG'
func main() {
    name := "kim"
    moved := name
    print(name)
}
MLG
expect_release_check_failure "use-after-move" 'use of moved value `name`'

cat >"$NEGATIVE_DIR/borrow-escape.mlg" <<'MLG'
func main() {
    name := "kim"
    print(take(con name))
}

func take(con name string) string {
    return name
}
MLG
expect_release_check_failure "borrow-escape" 'cannot move borrowed value `name`'

cat >"$NEGATIVE_DIR/overlap-borrow.mlg" <<'MLG'
type User struct {
    name string
    age int
}

func main() {
    mut user := User{name: "kim", age: 30}
    touch(mut user, con user)
}

func touch(mut left User, con right User) {
    left.age = right.age
}
MLG
expect_release_check_failure "overlap-borrow" 'borrow of `user` overlaps with an active borrow in this call'

ir_output="$("$RELEASE_BIN" ir examples/first.mlg)"
if [[ "$ir_output" != *"IrProgram {"* || "$ir_output" != *"IrFunction {"* || "$ir_output" != *"return_type: Unit"* ]]; then
  echo "release binary ir smoke failed" >&2
  echo "$ir_output" >&2
  exit 1
fi

run_command_output="$("$RELEASE_BIN" run examples/first.mlg)"
if [[ "$run_command_output" != "30" ]]; then
  echo "release binary run smoke failed: $run_command_output" >&2
  exit 1
fi

"$RELEASE_BIN" build examples/first.mlg -o "$SMOKE_BIN"
run_output="$("$SMOKE_BIN")"
if [[ "$run_output" != "30" ]]; then
  echo "release binary native build smoke failed: $run_output" >&2
  exit 1
fi

echo "release binary smoke passed"
