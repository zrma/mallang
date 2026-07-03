#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat <<'USAGE'
Usage:
  scripts/finalize-and-push.sh --verify-only
  scripts/finalize-and-push.sh --message "<type>: <summary>" [--bookmark main] [--no-push]

By default, writes a jj description with Codex attribution, runs the v0 RC
verification gate, verifies that the remote bookmark did not move before and
after verification, moves the bookmark, pushes it with jj, and verifies the
remote bookmark points at the published commit.

Use --no-push to run the same local finalization gate, including remote
freshness checks, without moving bookmarks or pushing to any remote.

Use --verify-only to run the v0 RC verification gate without changing the jj
description, moving bookmarks, or pushing to any remote.
USAGE
}

MESSAGE=""
BOOKMARK="main"
BOOKMARK_SET=0
REMOTE="origin"
PUSH=1
CHECK_REMOTE_FRESHNESS=1
VERIFY_ONLY=0
while [[ $# -gt 0 ]]; do
  case "$1" in
    --message)
      if [[ $# -lt 2 || -z "${2:-}" || "${2:-}" == --* ]]; then
        echo "--message requires a value" >&2
        usage >&2
        exit 2
      fi
      MESSAGE="${2:-}"
      shift 2
      ;;
    --bookmark)
      if [[ $# -lt 2 || -z "${2:-}" || "${2:-}" == --* ]]; then
        echo "--bookmark requires a value" >&2
        usage >&2
        exit 2
      fi
      BOOKMARK="${2:-}"
      BOOKMARK_SET=1
      shift 2
      ;;
    --no-push)
      PUSH=0
      shift
      ;;
    --verify-only)
      VERIFY_ONLY=1
      PUSH=0
      CHECK_REMOTE_FRESHNESS=0
      shift
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      echo "unknown option: $1" >&2
      usage >&2
      exit 2
      ;;
  esac
done

if [[ "$VERIFY_ONLY" -eq 1 ]]; then
  if [[ -n "$MESSAGE" ]]; then
    echo "--verify-only cannot be combined with --message" >&2
    exit 2
  fi
  if [[ "$BOOKMARK_SET" -eq 1 ]]; then
    echo "--verify-only cannot be combined with --bookmark" >&2
    exit 2
  fi
  scripts/verify-v0-rc.sh
  echo "finalize-and-push verify-only gate passed; no description, bookmarks, or remotes changed"
  jj status
  exit 0
fi

if [[ ! "$MESSAGE" =~ ^(feat|fix|perf|refactor|docs|test|build|ci|chore|revert):\ .+ ]]; then
  echo "invalid --message: expected '<type>: <summary>'" >&2
  exit 1
fi

read_commit_attribution() {
  sed -nE 's/^[[:space:]]*commit_attribution[[:space:]]*=[[:space:]]*"([^"]*)"[[:space:]]*$/\1/p' \
    "$HOME/.codex/config.toml" | head -n 1
}

run_jj_git() {
  if [[ -x /opt/homebrew/bin/git ]]; then
    PATH="/opt/homebrew/bin:$PATH" jj git "$@"
    return
  fi

  jj git "$@"
}

verify_description_attribution() {
  local attribution="$1"
  local trailer="Co-authored-by: $attribution"
  local description=""
  description="$(jj log -r @ --no-graph -T 'description')"
  DESC="$description" TRAILER="$trailer" python3 - <<'PYVERIFY'
import os
import sys

desc = os.environ["DESC"]
trailer = os.environ["TRAILER"]
lines = [line.rstrip() for line in desc.splitlines() if line.strip()]
count = sum(1 for line in lines if line == trailer)
if count != 1 or not lines or lines[-1] != trailer:
    print("attribution verification failed", file=sys.stderr)
    print(f"expected final trailer: {trailer}", file=sys.stderr)
    print(f"trailer count: {count}", file=sys.stderr)
    sys.exit(1)
PYVERIFY
}

describe_with_attribution() {
  local helper="$HOME/.codex/skills/vcs-jj/scripts/describe_with_attribution.sh"
  local attribution=""
  attribution="$(read_commit_attribution)"
  if [[ -z "$attribution" ]]; then
    echo "commit_attribution is not configured in $HOME/.codex/config.toml" >&2
    exit 1
  fi

  if [[ -x "$helper" ]] && "$helper" -r @ -- "$MESSAGE"; then
    verify_description_attribution "$attribution"
    return 0
  fi

  local normalized=""
  normalized="$(MESSAGE="$MESSAGE" TRAILER="Co-authored-by: $attribution" python3 - <<'PYNORMALIZE'
import os
import sys

message = os.environ["MESSAGE"].replace("\r\n", "\n").replace("\r", "\n")
trailer = os.environ["TRAILER"]
lines = [line.rstrip() for line in message.split("\n")]
lines = [line for line in lines if line != trailer]
while lines and not lines[-1].strip():
    lines.pop()
if not lines:
    print("message is empty after attribution normalization", file=sys.stderr)
    sys.exit(1)
print("\n".join(lines))
print()
print(trailer)
PYNORMALIZE
)"
  jj describe -r @ --message "$normalized"
  verify_description_attribution "$attribution"
}

verify_remote_bookmark_fresh() {
  local phase="$1"
  local local_commit=""
  local remote_commit=""
  local remote_ref="$BOOKMARK@$REMOTE"

  if ! local_commit="$(jj log -r "$BOOKMARK" --no-graph -T 'commit_id' 2>/dev/null)"; then
    echo "bookmark freshness check failed during $phase: local bookmark not found: $BOOKMARK" >&2
    exit 1
  fi

  run_jj_git fetch --remote "$REMOTE"

  if ! remote_commit="$(jj log -r "$remote_ref" --no-graph -T 'commit_id' 2>/dev/null)"; then
    echo "bookmark freshness check failed during $phase: remote bookmark not found: $remote_ref" >&2
    exit 1
  fi

  if [[ "$local_commit" != "$remote_commit" ]]; then
    echo "bookmark freshness check failed during $phase: $remote_ref moved since local $BOOKMARK" >&2
    echo "local $BOOKMARK: $local_commit" >&2
    echo "$remote_ref: $remote_commit" >&2
    echo "fetch/reconcile the local stack before publishing" >&2
    exit 1
  fi
}

verify_remote_bookmark_published() {
  local expected_commit="$1"
  local remote_commit=""
  local remote_ref="$BOOKMARK@$REMOTE"

  run_jj_git fetch --remote "$REMOTE"

  if ! remote_commit="$(jj log -r "$remote_ref" --no-graph -T 'commit_id' 2>/dev/null)"; then
    echo "publish verification failed: remote bookmark not found: $remote_ref" >&2
    exit 1
  fi

  if [[ "$remote_commit" != "$expected_commit" ]]; then
    echo "publish verification failed: $remote_ref does not point at published commit" >&2
    echo "expected: $expected_commit" >&2
    echo "$remote_ref: $remote_commit" >&2
    exit 1
  fi
}

if [[ "$CHECK_REMOTE_FRESHNESS" -eq 1 ]]; then
  verify_remote_bookmark_fresh "preflight"
fi

describe_with_attribution
scripts/verify-v0-rc.sh
if [[ "$PUSH" -eq 0 ]]; then
  if [[ "$CHECK_REMOTE_FRESHNESS" -eq 1 ]]; then
    verify_remote_bookmark_fresh "final"
  fi
  echo "finalize-and-push local gate passed; --no-push requested"
  jj status
  exit 0
fi

verify_remote_bookmark_fresh "final"
publish_target_commit="$(jj log -r @ --no-graph -T 'commit_id')"
jj bookmark set "$BOOKMARK" -r @
run_jj_git push --remote "$REMOTE" --bookmark "$BOOKMARK"
verify_remote_bookmark_published "$publish_target_commit"
jj bookmark list --all-remotes
jj status
