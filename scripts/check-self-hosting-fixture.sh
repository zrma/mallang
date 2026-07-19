#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

if [[ $# -ne 4 ]]; then
  echo "usage: scripts/check-self-hosting-fixture.sh <kind> <fixture> <stem> <profile>" >&2
  exit 2
fi

kind="$1"
fixture="$2"
stem="$3"
profile="$4"

: "${SELF_HOSTING_WORK:?SELF_HOSTING_WORK is required}"
: "${SELF_HOSTING_STAGE1:?SELF_HOSTING_STAGE1 is required}"
: "${SELF_HOSTING_ORACLE:?SELF_HOSTING_ORACLE is required}"
: "${SELF_HOSTING_ACCOUNTING:?SELF_HOSTING_ACCOUNTING is required}"
: "${SELF_HOSTING_SANITIZER:?SELF_HOSTING_SANITIZER is required}"

case "$profile" in
  stage1|strict|full)
    ;;
  *)
    echo "unknown self-hosting fixture profile: $profile" >&2
    exit 2
    ;;
esac

label=""
command=""
case "$kind" in
  lexer)
    label="lexer"
    ;;
  parser)
    label="parser"
    command="parse"
    ;;
  semantic)
    label="semantic"
    command="check"
    ;;
  ir)
    label="typed IR"
    command="ir"
    ;;
  ir-test)
    label="test typed IR"
    command="ir-test"
    ;;
  *)
    echo "unknown self-hosting differential kind: $kind" >&2
    exit 2
    ;;
esac

oracle_output="$SELF_HOSTING_WORK/$stem.oracle"
stage1_output="$SELF_HOSTING_WORK/$stem.stage1"
strict_output="$SELF_HOSTING_WORK/$stem.strict"
sanitizer_output="$SELF_HOSTING_WORK/$stem.sanitizer"
strict_stderr="$SELF_HOSTING_WORK/$stem.strict.stderr"
sanitizer_stderr="$SELF_HOSTING_WORK/$stem.sanitizer.stderr"

run_frontend() {
  local executable="$1"
  local stdout_path="$2"
  local stderr_path="${3:-}"
  local -a invocation=("$executable")
  if [[ -n "$command" ]]; then
    invocation+=("$command")
  fi
  invocation+=("$fixture")

  if [[ -n "$stderr_path" ]]; then
    "${invocation[@]}" >"$stdout_path" 2>"$stderr_path"
  else
    "${invocation[@]}" >"$stdout_path"
  fi
}

run_frontend "$SELF_HOSTING_ORACLE" "$oracle_output"
run_frontend "$SELF_HOSTING_STAGE1" "$stage1_output"

actual_outputs=("$stage1_output")
if [[ "$profile" != "stage1" ]]; then
  run_frontend "$SELF_HOSTING_ACCOUNTING" "$strict_output" "$strict_stderr"
  actual_outputs+=("$strict_output")
fi
if [[ "$profile" == "full" ]]; then
  run_frontend "$SELF_HOSTING_SANITIZER" "$sanitizer_output" "$sanitizer_stderr"
  actual_outputs+=("$sanitizer_output")
fi

for actual in "${actual_outputs[@]}"; do
  if ! cmp -s "$oracle_output" "$actual"; then
    echo "self-hosting $label differential mismatch: $stem" >&2
    diff -u "$oracle_output" "$actual" >&2 || true
    exit 1
  fi
done
if [[ "$profile" != "stage1" && -s "$strict_stderr" ]]; then
  echo "self-hosting $label runtime emitted stderr: $stem" >&2
  cat "$strict_stderr" >&2
  exit 1
fi
if [[ "$profile" == "full" && -s "$sanitizer_stderr" ]]; then
  echo "self-hosting $label runtime emitted stderr: $stem" >&2
  cat "$sanitizer_stderr" >&2
  exit 1
fi
