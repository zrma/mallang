#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

if [[ $# -ne 6 ]]; then
  echo "usage: scripts/check-self-hosting-compiler-pair-fixture.sh <stage1> <stage2> <kind> <fixture> <work> <stem>" >&2
  exit 2
fi

STAGE1="$1"
STAGE2="$2"
kind="$3"
fixture="$4"
work="$5"
stem="$6"

command=""
case "$kind" in
  lexer)
    ;;
  parser)
    command="parse"
    ;;
  semantic)
    command="check"
    ;;
  ir)
    command="ir"
    ;;
  ir-test)
    command="ir-test"
    ;;
  c)
    command="c"
    ;;
  *)
    echo "unknown self-hosting compiler-pair fixture kind: $kind" >&2
    exit 2
    ;;
esac

run_compiler() {
  local executable="$1"
  local label="$2"
  local -a invocation=("$executable")
  if [[ -n "$command" ]]; then
    invocation+=("$command")
  fi
  invocation+=("$fixture")

  set +e
  "${invocation[@]}" >"$work/$stem.$label.stdout" \
    2>"$work/$stem.$label.stderr"
  status=$?
  set -e
  printf '%s\n' "$status" >"$work/$stem.$label.status"
}

run_compiler "$STAGE1" stage1
run_compiler "$STAGE2" stage2

for suffix in stdout stderr status; do
  if ! cmp -s "$work/$stem.stage1.$suffix" "$work/$stem.stage2.$suffix"; then
    echo "self-hosting compiler-pair $kind mismatch: $fixture ($suffix)" >&2
    diff -u "$work/$stem.stage1.$suffix" "$work/$stem.stage2.$suffix" >&2 || true
    exit 1
  fi
done
