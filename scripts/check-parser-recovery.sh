#!/usr/bin/env bash
set -euo pipefail

if [[ $# -ne 1 ]]; then
  echo "usage: scripts/check-parser-recovery.sh <mlg-binary>" >&2
  exit 2
fi

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

MLG="$1"
CONSUMER="tests/fixtures/diagnostics/consume-jsonl.py"
PROJECT="tests/fixtures/diagnostics/multiple-project"
SOURCE="$PROJECT/src/a.mlg"
EXPECTED="$PROJECT/expected.stderr"
LEXICAL="tests/fixtures/diagnostics/lexical-errors.mlg"
OUT_DIR="target/mallang/parser-recovery"
mkdir -p "$OUT_DIR"

run_failure() {
  local label="$1"
  local count="$2"
  shift 2
  local json_stdout="$OUT_DIR/$label.json.stdout"
  local json_stderr="$OUT_DIR/$label.json.stderr"
  local human_stdout="$OUT_DIR/$label.human.stdout"
  local human_stderr="$OUT_DIR/$label.human.stderr"
  local rendered="$OUT_DIR/$label.rendered.stderr"
  local json_status
  local human_status

  set +e
  "$MLG" --diagnostic-format=json "$@" >"$json_stdout" 2>"$json_stderr"
  json_status=$?
  "$MLG" "$@" >"$human_stdout" 2>"$human_stderr"
  human_status=$?
  set -e

  if [[ "$json_status" -eq 0 ]] || [[ "$human_status" -eq 0 ]]; then
    echo "parser recovery $label unexpectedly succeeded" >&2
    exit 1
  fi
  if [[ -s "$json_stdout" ]] || [[ -s "$human_stdout" ]]; then
    echo "parser recovery $label emitted stdout" >&2
    exit 1
  fi

  python3 "$CONSUMER" --expect-stage frontend --expect-count "$count" \
    --expect-unique <"$json_stderr"
  python3 "$CONSUMER" --expect-stage frontend --expect-count "$count" \
    --expect-unique --render-human <"$json_stderr" >"$rendered"
  if ! cmp -s "$human_stderr" "$rendered"; then
    echo "parser recovery $label human/JSON parity mismatch" >&2
    exit 1
  fi
}

run_failure parse 3 parse "$SOURCE"
run_failure ir 3 ir "$SOURCE"
run_failure check 4 check "$PROJECT"
run_failure build 4 build "$PROJECT"
run_failure run 4 run "$PROJECT"
run_failure test 4 test "$PROJECT"

for label in check build run test; do
  if ! cmp -s "$EXPECTED" "$OUT_DIR/$label.human.stderr"; then
    echo "parser recovery $label stable diagnostic order mismatch" >&2
    exit 1
  fi
done

run_failure lexical 1 parse "$LEXICAL"

CAP_SOURCE="$OUT_DIR/cap.mlg"
{
  printf 'func main() {\n'
  for index in $(seq 1 40); do
    printf '    broken%s := ;\n' "$index"
  done
  printf '}\n'
} >"$CAP_SOURCE"
run_failure cap 32 parse "$CAP_SOURCE"
python3 "$CONSUMER" --expect-stage frontend --expect-count 32 \
  --expect-unique --expect-line-range 2:33 <"$OUT_DIR/cap.json.stderr"

echo "parser recovery command parity, lexical fail-fast, deduplication, and cap acceptance passed"
