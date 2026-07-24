#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
WORK_DIR="$(mktemp -d "${TMPDIR:-/tmp}/mallang-work-packets.XXXXXX")"
trap 'rm -rf "$WORK_DIR"' EXIT
EXISTING_ID="existing"
SAMPLE_ID="sample"
UNKNOWN_ID="unknown"

mkdir -p "$WORK_DIR/scripts" "$WORK_DIR/docs/artifacts/$EXISTING_ID"
cp \
  "$ROOT/scripts/archive-work.sh" \
  "$ROOT/scripts/check-todo-state.py" \
  "$ROOT/scripts/start-work.sh" \
  "$WORK_DIR/scripts/"

cat >"$WORK_DIR/docs/TODO_INDEX.md" <<'EOF'
# Work Packet Index

## Active

현재 활성 패킷 없음.
EOF

cat >"$WORK_DIR/docs/artifacts/$EXISTING_ID/spec.md" <<'EOF'
# Existing Artifact

Status: complete
EOF

expect_failure() {
  local label="$1"
  local expected="$2"
  shift 2

  if "$@" >"$WORK_DIR/$label.log" 2>&1; then
    echo "$label failed: expected non-zero exit" >&2
    exit 1
  fi
  if ! grep -Fq "$expected" "$WORK_DIR/$label.log"; then
    echo "$label failed: missing output pattern: $expected" >&2
    exit 1
  fi
}

cd "$WORK_DIR"

expect_failure \
  completed-id-reuse \
  "work id is already completed" \
  scripts/start-work.sh --work-id "$EXISTING_ID"

scripts/start-work.sh --work-id "$SAMPLE_ID" >/dev/null
expect_failure \
  active-archive \
  "work packet is not complete" \
  scripts/archive-work.sh --work-id "$SAMPLE_ID"

perl -0pi -e 's/Status: active/Status: complete/; s/\| C1 \| todo \|/| C1 | done |/' \
  docs/todo-sample/spec.md
scripts/archive-work.sh --work-id "$SAMPLE_ID" >/dev/null

[[ -f "docs/artifacts/$SAMPLE_ID/spec.md" ]]
[[ ! -e "docs/todo-$SAMPLE_ID" ]]
python3 scripts/check-todo-state.py >/dev/null

cat >README.md <<'EOF'
Stale link: docs/todo-sample/spec.md
EOF
expect_failure \
  stale-completed-reference \
  "references completed work as docs/todo-sample" \
  python3 scripts/check-todo-state.py

cat >README.md <<EOF
Broken link: docs/artifacts/$UNKNOWN_ID/spec.md
EOF
expect_failure \
  unknown-artifact-reference \
  "references unknown artifact docs/artifacts/$UNKNOWN_ID" \
  python3 scripts/check-todo-state.py

echo "work packet lifecycle checks passed"
