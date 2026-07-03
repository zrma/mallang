#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

VERIFY_DEEP=1

usage() {
  cat <<'USAGE'
Usage: scripts/verify-v0-rc.sh [--skip-deep-sanitizers]

Runs the local v0 release-candidate verification gate without moving bookmarks
or pushing to any remote.
USAGE
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --skip-deep-sanitizers)
      VERIFY_DEEP=0
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

read_commit_attribution() {
  sed -nE 's/^[[:space:]]*commit_attribution[[:space:]]*=[[:space:]]*"([^"]*)"[[:space:]]*$/\1/p' \
    "$HOME/.codex/config.toml" | head -n 1
}

verify_checked_roadmaps() {
  local path
  for path in ROADMAP.md docs/ROADMAP.md; do
    if grep -Fq '[ ]' "$path"; then
      echo "$path contains unchecked roadmap items" >&2
      grep -nF '[ ]' "$path" >&2
      exit 1
    fi
  done
}

verify_no_empty_local_stack_changes() {
  if jj log -r 'main..@ & empty()' --no-graph -T 'change_id.short() ++ " " ++ description.first_line() ++ "\n"' |
    grep -q .; then
    echo "local stack contains empty changes" >&2
    jj log -r 'main..@ & empty()' --no-graph \
      -T 'change_id.short() ++ " " ++ description.first_line() ++ "\n"' >&2
    exit 1
  fi
}

verify_codex_attribution() {
  local attribution trailer change_id description
  attribution="$(read_commit_attribution)"
  if [[ -z "$attribution" ]]; then
    echo "commit_attribution is not configured in $HOME/.codex/config.toml" >&2
    exit 1
  fi
  trailer="Co-authored-by: $attribution"
  while IFS= read -r change_id; do
    [[ -z "$change_id" ]] && continue
    description="$(jj log -r "$change_id" --no-graph -T 'description')"
    CHANGE_ID="$change_id" DESC="$description" TRAILER="$trailer" python3 - <<'PYVERIFY'
import os
import sys

change_id = os.environ["CHANGE_ID"]
desc = os.environ["DESC"]
trailer = os.environ["TRAILER"]

lines = [line.rstrip() for line in desc.splitlines() if line.strip()]
count = sum(1 for line in lines if line == trailer)
if count != 1 or not lines or lines[-1] != trailer:
    print("Codex attribution verification failed for local stack:", file=sys.stderr)
    print(f"  {change_id}", file=sys.stderr)
    print(f"expected final trailer: {trailer}", file=sys.stderr)
    sys.exit(1)
PYVERIFY
  done < <(jj log -r 'main..@' --no-graph -T 'change_id.short() ++ "\n"')
}

scripts/check-release-helpers.sh
scripts/check.sh
if [[ "$VERIFY_DEEP" -eq 1 ]]; then
  scripts/check-generated-c-sanitizers.sh --assume-generated
fi
verify_checked_roadmaps
verify_no_empty_local_stack_changes
verify_codex_attribution

echo "v0 release-candidate local verification passed"
