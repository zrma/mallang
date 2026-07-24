#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat <<'USAGE'
Usage: scripts/archive-work.sh --work-id <id>

Moves one completed docs/todo-<id> packet to docs/artifacts/<id>.
USAGE
}

WORK_ID=""
while [[ $# -gt 0 ]]; do
  case "$1" in
    --work-id)
      WORK_ID="${2:-}"
      shift 2
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

if [[ ! "$WORK_ID" =~ ^[a-z0-9]+(-[a-z0-9]+)*$ ]]; then
  echo "invalid or missing --work-id: $WORK_ID" >&2
  exit 1
fi

TODO_DIR="docs/todo-$WORK_ID"
ARTIFACT_DIR="docs/artifacts/$WORK_ID"
SPEC="$TODO_DIR/spec.md"

if [[ ! -f "$SPEC" ]]; then
  echo "work packet does not exist: $TODO_DIR" >&2
  exit 1
fi
if [[ -e "$ARTIFACT_DIR" ]]; then
  echo "work artifact already exists: $ARTIFACT_DIR" >&2
  exit 1
fi
if ! grep -Eq '^Status: complete(; .+)?$' "$SPEC"; then
  echo "work packet is not complete: $SPEC" >&2
  exit 1
fi
if grep -Eq '^- \[ \]' "$SPEC"; then
  echo "work packet has unchecked tasks: $SPEC" >&2
  exit 1
fi
if grep -Eq '^\| C[^|]*\| (todo|pending|blocked|in progress) \|' "$SPEC"; then
  echo "work packet has unfinished checklist rows: $SPEC" >&2
  exit 1
fi

mkdir -p docs/artifacts
mv "$TODO_DIR" "$ARTIFACT_DIR"

echo "archived $TODO_DIR as $ARTIFACT_DIR"
echo "update repository references, then run: python3 scripts/check-todo-state.py"
