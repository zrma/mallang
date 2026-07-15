#!/usr/bin/env bash
set -euo pipefail

if [[ $# -ne 1 ]]; then
  echo "usage: scripts/check-diagnostics.sh <mlg-binary>" >&2
  exit 2
fi

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

MLG="$1"
CONSUMER="tests/fixtures/diagnostics/consume-jsonl.py"
OUT_DIR="target/mallang/diagnostics"
mkdir -p "$OUT_DIR"

run_failure() {
  local label="$1"
  local stage="$2"
  shift 2
  local stdout_path="$OUT_DIR/$label.stdout"
  local stderr_path="$OUT_DIR/$label.stderr"
  local status

  set +e
  "$@" >"$stdout_path" 2>"$stderr_path"
  status=$?
  set -e
  if [[ "$status" -eq 0 ]]; then
    echo "diagnostic $label unexpectedly succeeded" >&2
    exit 1
  fi
  if [[ -s "$stdout_path" ]]; then
    echo "diagnostic $label emitted stdout" >&2
    exit 1
  fi
  python3 "$CONSUMER" --expect-stage "$stage" <"$stderr_path"
}

run_failure "cli" "cli" "$MLG" --diagnostic-format=json nope
run_failure \
  "input" \
  "input" \
  "$MLG" --diagnostic-format json check "$OUT_DIR/missing.mlg"
run_failure \
  "frontend" \
  "frontend" \
  "$MLG" --diagnostic-format=json check tests/fixtures/diagnostics/frontend.mlg
run_failure \
  "package" \
  "package" \
  "$MLG" --diagnostic-format=json check tests/fixtures/project-cycle
run_failure \
  "link" \
  "link" \
  "$MLG" --diagnostic-format=json check tests/fixtures/diagnostics/link-project
run_failure \
  "semantic" \
  "semantic" \
  "$MLG" --diagnostic-format=json check tests/fixtures/invalid-v05-ownership/use-after-move.mlg
run_failure \
  "dependency" \
  "semantic" \
  "$MLG" --diagnostic-format=json check tests/fixtures/diagnostics/dependency-project/app
python3 "$CONSUMER" \
  --expect-stage semantic \
  --expect-path diagcore/src/value.mlg \
  <"$OUT_DIR/dependency.stderr"

set +e
"$MLG" check tests/fixtures/invalid-v05-ownership/use-after-move.mlg \
  >"$OUT_DIR/semantic-human.stdout" 2>"$OUT_DIR/semantic-human.stderr"
human_status=$?
set -e
if [[ "$human_status" -eq 0 ]] || [[ -s "$OUT_DIR/semantic-human.stdout" ]]; then
  echo "human diagnostic parity setup failed" >&2
  exit 1
fi
python3 "$CONSUMER" --expect-stage semantic --render-human \
  <"$OUT_DIR/semantic.stderr" >"$OUT_DIR/semantic-rendered-human.stderr"
if ! cmp -s "$OUT_DIR/semantic-human.stderr" "$OUT_DIR/semantic-rendered-human.stderr"; then
  echo "human and JSON diagnostic rendering diverged" >&2
  exit 1
fi

FMT_PROJECT="$OUT_DIR/fmt-project"
rm -rf "$FMT_PROJECT"
mkdir -p "$FMT_PROJECT/src/util"
printf '[project]\nname = "fmtfixture"\n' >"$FMT_PROJECT/mallang.toml"
printf 'func main(){print(1)}\n' >"$FMT_PROJECT/src/main.mlg"
printf 'package util\nfunc Value()int{return 1}\n' >"$FMT_PROJECT/src/util/value.mlg"
set +e
"$MLG" --diagnostic-format=json fmt --check "$FMT_PROJECT" \
  >"$OUT_DIR/fmt.stdout" 2>"$OUT_DIR/fmt.stderr"
fmt_status=$?
set -e
if [[ "$fmt_status" -eq 0 ]] || [[ -s "$OUT_DIR/fmt.stdout" ]]; then
  echo "formatter JSON diagnostics unexpectedly succeeded or emitted stdout" >&2
  exit 1
fi
python3 "$CONSUMER" --expect-stage input --expect-count 2 <"$OUT_DIR/fmt.stderr"

set +e
"$MLG" --diagnostic-format=json test tests/fixtures/project-test-failure \
  >"$OUT_DIR/test.stdout" 2>"$OUT_DIR/test.stderr"
test_status=$?
set -e
if [[ "$test_status" -eq 0 ]] || \
  ! grep -Fq 'test result: FAILED. 2 passed; 1 failed' "$OUT_DIR/test.stdout"; then
  echo "test JSON diagnostic setup failed" >&2
  exit 1
fi
python3 "$CONSUMER" --expect-stage native <"$OUT_DIR/test.stderr"

FAKE_BIN="$OUT_DIR/fake-bin"
mkdir -p "$FAKE_BIN"
printf '#!/usr/bin/env sh\nprintf "synthetic native failure\\n" >&2\nexit 7\n' \
  >"$FAKE_BIN/clang"
chmod +x "$FAKE_BIN/clang"
run_failure \
  "native" \
  "native" \
  env PATH="$FAKE_BIN:/usr/bin:/bin" \
  "$MLG" --diagnostic-format=json build examples/first.mlg -o "$OUT_DIR/native-output"

if ! "$MLG" --diagnostic-format=json check examples/first.mlg \
  >"$OUT_DIR/success.stdout" 2>"$OUT_DIR/success.stderr"; then
  echo "successful JSON-mode check failed" >&2
  exit 1
fi
if [[ "$(cat "$OUT_DIR/success.stdout")" != 'examples/first.mlg: ok' ]] || \
  [[ -s "$OUT_DIR/success.stderr" ]]; then
  echo "successful JSON mode changed ordinary command output" >&2
  exit 1
fi

if ! "$MLG" --diagnostic-format=json --help \
  >"$OUT_DIR/help.stdout" 2>"$OUT_DIR/help.stderr" || \
  [[ -s "$OUT_DIR/help.stderr" ]] || \
  ! grep -Fq -- '--diagnostic-format <human|json>' "$OUT_DIR/help.stdout"; then
  echo "diagnostic format help contract mismatch" >&2
  exit 1
fi

echo "machine-readable diagnostic schema, parity, and JSONL consumer smoke passed"
