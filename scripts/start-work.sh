#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat <<'USAGE'
Usage: scripts/start-work.sh --work-id <id>

Creates docs/todo-<id>/spec.md and open-questions.md if they do not exist.
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

if [[ -e "$ARTIFACT_DIR" ]]; then
  echo "work id is already completed: $ARTIFACT_DIR" >&2
  exit 1
fi

mkdir -p "$TODO_DIR"

if [[ ! -f "$TODO_DIR/spec.md" ]]; then
  cat >"$TODO_DIR/spec.md" <<EOF
# Spec: $WORK_ID

Status: active

## 목표

- TODO

## 범위

- TODO

## C-체크리스트

| ID | 상태 | Verify command | 작업 항목 |
| --- | --- | --- | --- |
| C1 | todo | \`scripts/check.sh\` | TODO |

## 완료 기준

- C-체크리스트가 완료되고 검증 명령이 통과한다.
EOF
fi

if [[ ! -f "$TODO_DIR/open-questions.md" ]]; then
  cat >"$TODO_DIR/open-questions.md" <<'EOF'
# Open Questions

현재 미결 항목 없음.
EOF
fi

echo "initialized $TODO_DIR"
