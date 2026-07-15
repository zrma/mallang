#!/usr/bin/env bash
set -euo pipefail

if [[ $# -ne 1 ]]; then
  echo "usage: scripts/check-hardening-corpus.sh <mlg-binary>" >&2
  exit 2
fi

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

mlg="$1"
consumer="tests/fixtures/diagnostics/consume-jsonl.py"
corpus="tests/fixtures/hardening/crash-corpus"
work="target/mallang/hardening-corpus"
mkdir -p "$work"

expected_files="$work/expected-files.txt"
actual_files="$work/actual-files.txt"
cat >"$expected_files" <<'FILES'
frontend-missing-parameter.mlg
link-invalid-receiver.mlg
ownership-borrow-return.mlg
ownership-use-after-move.mlg
package-unresolved-import.mlg
semantic-empty-match.mlg
FILES
find "$corpus" -maxdepth 1 -type f -name '*.mlg' -exec basename {} \; | LC_ALL=C sort \
  >"$actual_files"
if ! cmp -s "$expected_files" "$actual_files"; then
  echo "hardening crash corpus registration mismatch" >&2
  diff -u "$expected_files" "$actual_files" >&2 || true
  exit 1
fi

check_case() {
  local file="$1"
  local stage="$2"
  local message="$3"
  local stdout_path="$work/$file.stdout"
  local stderr_path="$work/$file.stderr"
  local exit_code

  set +e
  "$mlg" --diagnostic-format json check "$corpus/$file" \
    >"$stdout_path" 2>"$stderr_path"
  exit_code=$?
  set -e

  if [[ "$exit_code" -eq 0 ]] || [[ -s "$stdout_path" ]]; then
    echo "hardening corpus case did not fail cleanly: $file" >&2
    exit 1
  fi
  python3 "$consumer" --expect-stage "$stage" <"$stderr_path"
  if ! grep -Fq "\"message\":\"$message" "$stderr_path"; then
    echo "hardening corpus message mismatch: $file" >&2
    cat "$stderr_path" >&2
    exit 1
  fi
}

check_case frontend-missing-parameter.mlg frontend "expected parameter name"
check_case package-unresolved-import.mlg package "unresolved import"
check_case link-invalid-receiver.mlg link "method receiver type must be declared in the same package"
check_case semantic-empty-match.mlg semantic "match requires at least one arm"
check_case ownership-borrow-return.mlg semantic "cannot move borrowed value"
check_case ownership-use-after-move.mlg semantic "use of moved value"

echo "hardening crash corpus CLI diagnostics passed"
