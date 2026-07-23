#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

CARGO=(cargo)
if [[ -n "${CARGO_BIN:-}" ]]; then
  CARGO=("$CARGO_BIN")
fi

RELEASE_BIN="target/release/mlg"
RELEASE_COMPILER="target/release/mlgc"
SMOKE_BIN="target/mallang/release-binary-first"
NEGATIVE_DIR="target/mallang/release-binary-negative"

mkdir -p target/mallang "$NEGATIVE_DIR"

expect_release_command_failure() {
  local label="$1"
  local expected_stderr="$2"
  shift 2

  local stdout="$NEGATIVE_DIR/$label.stdout"
  local stderr="$NEGATIVE_DIR/$label.stderr"

  if "$@" >"$stdout" 2>"$stderr"; then
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

expect_release_check_failure() {
  local label="$1"
  local expected_stderr="$2"
  local source="$NEGATIVE_DIR/$label.mlg"

  expect_release_command_failure "$label" "$expected_stderr" "$RELEASE_BIN" check "$source"
}

"${CARGO[@]}" build --release --locked --bin mlg
scripts/build-self-hosted-compiler.sh \
  --stage0 "$RELEASE_BIN" \
  --output "$RELEASE_COMPILER" \
  >/dev/null

crate_version="$(sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml)"
version_output="$("$RELEASE_BIN" --version)"
if [[ "$version_output" != "mlg $crate_version" ]]; then
  echo "release binary version smoke failed: expected mlg $crate_version, got '$version_output'" >&2
  exit 1
fi
if [[ "$("$RELEASE_COMPILER" --version)" != "mlgc protocol 1" ]]; then
  echo "release compiler protocol smoke failed" >&2
  exit 1
fi
expected_provenance=$'mlg '"$crate_version"$'\ndriver: rust\ncompiler: self\ncore: mlgc protocol 1'
if [[ "$("$RELEASE_BIN" --version --verbose)" != "$expected_provenance" ]]; then
  echo "release default compiler provenance smoke failed" >&2
  exit 1
fi
if [[ "$("$RELEASE_BIN" --compiler stage0 --version --verbose)" != \
  $'mlg '"$crate_version"$'\ndriver: rust\ncompiler: stage0\ncore: rust-stage0' ]]; then
  echo "release Stage0 rollback provenance smoke failed" >&2
  exit 1
fi

expect_release_command_failure "no-args" "usage:" "$RELEASE_BIN"
expect_release_command_failure "unknown-command" 'unknown subcommand `nope`' "$RELEASE_BIN" nope
expect_release_command_failure \
  "build-missing-output" \
  "missing value for -o/--output" \
  "$RELEASE_BIN" build examples/first.mlg -o
expect_release_command_failure \
  "build-unknown-argument" \
  'unknown build argument `--wat`' \
  "$RELEASE_BIN" build examples/first.mlg --wat

help_output="$("$RELEASE_BIN" --help)"
if [[ "$help_output" != *"usage:"* || \
  "$help_output" != *"$RELEASE_BIN check <input>"* || \
  "$help_output" != *"$RELEASE_BIN ir <input>"* || \
  "$help_output" != *"$RELEASE_BIN fmt [--check] <input>"* || \
  "$help_output" != *"$RELEASE_BIN run <input> [-- <program-args>...]"* || \
  "$help_output" != *"$RELEASE_BIN test <input> [--exact <test-id>]"* || \
  "$help_output" != *"$RELEASE_BIN --version"* ]]; then
  echo "release binary help smoke failed" >&2
  echo "$help_output" >&2
  exit 1
fi

scripts/check-formatter.sh "$RELEASE_BIN"
scripts/check-test-workflow.sh "$RELEASE_BIN"
scripts/check-path-dependencies.sh "$RELEASE_BIN"
scripts/check-diagnostics.sh "$RELEASE_BIN"
scripts/check-parser-recovery.sh "$RELEASE_BIN"
scripts/check-hardening-corpus.sh "$RELEASE_BIN"

lex_output="$("$RELEASE_BIN" lex examples/first.mlg)"
if [[ "$lex_output" != T\|Keyword.Func\|0\|4\|$'\n'* || \
  "$lex_output" != *$'\n'T\|Ident\|67\|70\|97,100,100$'\n'* ]]; then
  echo "release binary lex smoke failed" >&2
  echo "$lex_output" >&2
  exit 1
fi

parse_output="$("$RELEASE_BIN" parse examples/first.mlg)"
if [[ "$parse_output" != N\|0\|Program\|0\|0\|110\|\|3$'\n'* || \
  "$parse_output" != *$'\n'N\|1\|FunctionDecl.Package\|0\|0\|60\|109,97,105,110\|1$'\n'* ]]; then
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
if [[ "$ir_output" != IR\|2$'\n'FUNCTION\|main\|unit\|0\|3$'\n'* ]] || \
   [[ "$ir_output" != *$'\n'FUNCTION\|add\|int\|2\|1$'\n'* ]] || \
   [[ "$ir_output" != *$'\n'CLOSURES\|0 ]]; then
  echo "release binary ir smoke failed" >&2
  echo "$ir_output" >&2
  exit 1
fi

run_command_output="$("$RELEASE_BIN" run examples/first.mlg)"
if [[ "$run_command_output" != "30" ]]; then
  echo "release binary run smoke failed: $run_command_output" >&2
  exit 1
fi

process_args_output="$(
  "$RELEASE_BIN" run tests/fixtures/v06-process-io/args.mlg -- alpha 한
)"
if [[ "$process_args_output" != $'3\nalpha\n한' ]]; then
  echo "release binary process argument smoke failed: $process_args_output" >&2
  exit 1
fi

if "$RELEASE_BIN" run tests/fixtures/v06-process-io/exit.mlg \
  >"$NEGATIVE_DIR/process-exit.stdout" 2>"$NEGATIVE_DIR/process-exit.stderr"; then
  echo "release binary process exit smoke unexpectedly succeeded" >&2
  exit 1
else
  process_exit_status=$?
fi
if [[ "$process_exit_status" -ne 7 ]] || \
  [[ -s "$NEGATIVE_DIR/process-exit.stdout" ]] || \
  [[ -s "$NEGATIVE_DIR/process-exit.stderr" ]]; then
  echo "release binary process exit smoke failed" >&2
  exit 1
fi

printf 'release-file-한\0text' >"$NEGATIVE_DIR/file-input.txt"
if ! "$RELEASE_BIN" run examples/file-io.mlg -- \
  "$NEGATIVE_DIR/file-input.txt" "$NEGATIVE_DIR/file-output.txt" \
  >"$NEGATIVE_DIR/file-run.stdout" 2>"$NEGATIVE_DIR/file-run.stderr"; then
  echo "release binary file I/O smoke failed" >&2
  cat "$NEGATIVE_DIR/file-run.stderr" >&2
  exit 1
fi
if [[ -s "$NEGATIVE_DIR/file-run.stdout" ]] || \
  [[ -s "$NEGATIVE_DIR/file-run.stderr" ]] || \
  ! cmp -s "$NEGATIVE_DIR/file-input.txt" "$NEGATIVE_DIR/file-output.txt"; then
  echo "release binary file I/O output mismatch" >&2
  exit 1
fi

printf 'INFO first\nERROR 둘\nERROR final' >"$NEGATIVE_DIR/streaming-input.txt"
streaming_output="$(
  "$RELEASE_BIN" run tests/fixtures/v11-streaming-io/for-each-line.mlg -- \
    "$NEGATIVE_DIR/streaming-input.txt" ERROR
)"
if [[ "$streaming_output" != $'2\nERROR 둘\n3\nERROR final\n2' ]]; then
  echo "release binary streaming file I/O smoke failed: $streaming_output" >&2
  exit 1
fi

collections_output="$("$RELEASE_BIN" run examples/collections-map.mlg)"
if [[ "$collections_output" != $'inserted\n1\n1\nKim\n2\ntrue\ntrue\nKim\n3\ntrue\nfalse\nKim\n3\n0' ]]; then
  echo "release binary collections Map smoke failed: $collections_output" >&2
  exit 1
fi

textstats_output="$(
  "$RELEASE_BIN" run examples/projects/textstats -- \
    tests/fixtures/v06-reference-cli/input.txt
)"
if [[ "$textstats_output" != $'bytes=12\nscalars=10\nlines=4\ndistinct_line_lengths=3' ]]; then
  echo "release binary reference CLI smoke failed: $textstats_output" >&2
  exit 1
fi

"$RELEASE_BIN" build examples/first.mlg -o "$SMOKE_BIN"
run_output="$("$SMOKE_BIN")"
if [[ "$run_output" != "30" ]]; then
  echo "release binary native build smoke failed: $run_output" >&2
  exit 1
fi

echo "release binary smoke passed"
