#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

LOG_DIR="target/mallang/release-helper-checks"
mkdir -p "$LOG_DIR"

expect_status() {
  local label="$1"
  local expected="$2"
  shift 2

  local output="$LOG_DIR/$label.log"
  local status=0
  set +e
  "$@" >"$output" 2>&1
  status=$?
  set -e

  if [[ "$status" -ne "$expected" ]]; then
    echo "$label failed: expected exit $expected, got $status" >&2
    tail -n 40 "$output" >&2
    exit 1
  fi
}

expect_log_contains() {
  local label="$1"
  local pattern="$2"
  local output="$LOG_DIR/$label.log"
  if ! grep -Fq -- "$pattern" "$output"; then
    echo "$label failed: missing output pattern: $pattern" >&2
    tail -n 40 "$output" >&2
    exit 1
  fi
}

expect_file_contains() {
  local path="$1"
  local pattern="$2"
  if ! grep -Fq -- "$pattern" "$path"; then
    echo "$path failed: missing source pattern: $pattern" >&2
    exit 1
  fi
}

check_shell_syntax() {
  local script
  for script in "$@"; do
    bash -n "$script"
  done
}

check_shell_syntax \
  scripts/check-generated-c-sanitizers.sh \
  scripts/check-release-helpers.sh \
  scripts/check.sh \
  scripts/finalize-and-push.sh \
  scripts/start-work.sh \
  scripts/verify-v0-rc.sh

expect_status finalize_help 0 scripts/finalize-and-push.sh --help
expect_log_contains finalize_help "scripts/finalize-and-push.sh --verify-only"
expect_log_contains finalize_help "--no-push"
expect_log_contains finalize_help "remote bookmark did not move"
expect_log_contains finalize_help "including remote"
expect_log_contains finalize_help "freshness checks"
expect_file_contains scripts/finalize-and-push.sh 'PATH="/opt/homebrew/bin:$PATH" jj git "$@"'
expect_file_contains scripts/finalize-and-push.sh 'run_jj_git fetch --remote "$REMOTE"'
expect_file_contains scripts/finalize-and-push.sh 'CHECK_REMOTE_FRESHNESS=1'
expect_file_contains scripts/finalize-and-push.sh 'CHECK_REMOTE_FRESHNESS=0'
expect_file_contains scripts/finalize-and-push.sh 'verify_remote_bookmark_fresh "preflight"'
expect_file_contains scripts/finalize-and-push.sh 'verify_remote_bookmark_fresh "final"'
expect_file_contains scripts/finalize-and-push.sh 'run_jj_git push --remote "$REMOTE" --bookmark "$BOOKMARK"'

expect_status verify_only_message 2 \
  scripts/finalize-and-push.sh --verify-only --message "test: publish v0 release candidate"
expect_log_contains verify_only_message "--verify-only cannot be combined with --message"

expect_status verify_only_bookmark 2 \
  scripts/finalize-and-push.sh --verify-only --bookmark main
expect_log_contains verify_only_bookmark "--verify-only cannot be combined with --bookmark"

expect_status missing_message 2 scripts/finalize-and-push.sh --message
expect_log_contains missing_message "--message requires a value"

expect_status empty_message 2 scripts/finalize-and-push.sh --message ""
expect_log_contains empty_message "--message requires a value"

expect_status message_next_option 2 scripts/finalize-and-push.sh --message --no-push
expect_log_contains message_next_option "--message requires a value"

expect_status missing_bookmark 2 scripts/finalize-and-push.sh --bookmark
expect_log_contains missing_bookmark "--bookmark requires a value"

expect_status empty_bookmark 2 scripts/finalize-and-push.sh --bookmark ""
expect_log_contains empty_bookmark "--bookmark requires a value"

expect_status bookmark_next_option 2 scripts/finalize-and-push.sh --bookmark --no-push
expect_log_contains bookmark_next_option "--bookmark requires a value"

expect_status invalid_message 1 scripts/finalize-and-push.sh --message "publish v0"
expect_log_contains invalid_message "invalid --message"

expect_status unknown_option 2 scripts/finalize-and-push.sh --unknown-option
expect_log_contains unknown_option "unknown option: --unknown-option"

echo "release helper contract checks passed"
