#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat <<'USAGE'
Usage: scripts/finalize-and-push.sh --message "<type>: <summary>" [--bookmark main] [--no-push]

Writes a jj description with Codex attribution, runs the v0 RC verification
gate, moves the bookmark, and pushes it with jj.

Use --no-push to run the same local finalization gate without moving bookmarks
or pushing to any remote.
USAGE
}

MESSAGE=""
BOOKMARK="main"
PUSH=1
while [[ $# -gt 0 ]]; do
  case "$1" in
    --message)
      MESSAGE="${2:-}"
      shift 2
      ;;
    --bookmark)
      BOOKMARK="${2:-}"
      shift 2
      ;;
    --no-push)
      PUSH=0
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

if [[ ! "$MESSAGE" =~ ^(feat|fix|perf|refactor|docs|test|build|ci|chore|revert):\ .+ ]]; then
  echo "invalid --message: expected '<type>: <summary>'" >&2
  exit 1
fi

read_commit_attribution() {
  sed -nE 's/^[[:space:]]*commit_attribution[[:space:]]*=[[:space:]]*"([^"]*)"[[:space:]]*$/\1/p' \
    "$HOME/.codex/config.toml" | head -n 1
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

describe_with_attribution
scripts/verify-v0-rc.sh
if [[ "$PUSH" -eq 0 ]]; then
  echo "finalize-and-push local gate passed; --no-push requested"
  jj status
  exit 0
fi

jj bookmark set "$BOOKMARK" -r @
jj git push --remote origin --bookmark "$BOOKMARK"
jj bookmark list --all-remotes
jj status
