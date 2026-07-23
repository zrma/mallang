#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

driver="${1:-target/debug/mlg}"
self_compiler="${2:-target/debug/mlgc}"
fixture="tests/fixtures/naming-lint/violations.mlg"
valid_fixture="tests/fixtures/naming-lint/valid.mlg"
project_fixture="tests/fixtures/naming-lint-project"
output_root="target/mallang/naming-lint"

mkdir -p "$output_root"

run_stage0() {
  "$driver" --compiler stage0 "$@"
}

run_self() {
  "$driver" --compiler self --self-compiler "$self_compiler" "$@"
}

run_stage0 lint "$fixture" \
  >"$output_root/stage0.stdout" \
  2>"$output_root/stage0.stderr"
run_self lint "$fixture" \
  >"$output_root/self.stdout" \
  2>"$output_root/self.stderr"

if [[ -s "$output_root/stage0.stdout" || -s "$output_root/self.stdout" ]]; then
  echo "naming lint emitted unexpected stdout" >&2
  exit 1
fi
cmp "$output_root/stage0.stderr" "$output_root/self.stderr"
if [[ "$(wc -l <"$output_root/stage0.stderr" | tr -d ' ')" != "10" ]]; then
  echo "naming lint expected 10 warnings" >&2
  exit 1
fi
for rule in MLG-NAME-001 MLG-NAME-002 MLG-NAME-003 MLG-NAME-004 MLG-NAME-005 MLG-NAME-006 MLG-NAME-007 MLG-NAME-008; do
  if ! grep -Fq "warning[$rule]" "$output_root/stage0.stderr"; then
    echo "naming lint did not emit $rule" >&2
    exit 1
  fi
done

run_stage0 --diagnostic-format json lint "$fixture" \
  >"$output_root/stage0-json.stdout" \
  2>"$output_root/stage0.jsonl"
run_self --diagnostic-format json lint "$fixture" \
  >"$output_root/self-json.stdout" \
  2>"$output_root/self.jsonl"
cmp "$output_root/stage0.jsonl" "$output_root/self.jsonl"
tests/fixtures/diagnostics/consume-jsonl.py \
  --expect-stage lint \
  --expect-severity warning \
  --expect-code-prefix MLG-NAME- \
  --expect-count 10 \
  --expect-path "$fixture" \
  <"$output_root/stage0.jsonl"

if run_stage0 lint --deny-warnings "$fixture" >/dev/null 2>"$output_root/deny.stderr"; then
  echo "naming lint --deny-warnings accepted violations" >&2
  exit 1
fi
run_stage0 lint --allow MLG-NAME-004 "$fixture" \
  >"$output_root/allow.stdout" \
  2>"$output_root/allow.stderr"
if grep -Fq "warning[MLG-NAME-004]" "$output_root/allow.stderr"; then
  echo "naming lint --allow did not suppress its rule" >&2
  exit 1
fi

run_stage0 lint "$valid_fixture" \
  >"$output_root/valid-stage0.stdout" \
  2>"$output_root/valid-stage0.stderr"
run_self lint "$valid_fixture" \
  >"$output_root/valid-self.stdout" \
  2>"$output_root/valid-self.stderr"
if [[ -s "$output_root/valid-stage0.stdout" || -s "$output_root/valid-stage0.stderr" || \
      -s "$output_root/valid-self.stdout" || -s "$output_root/valid-self.stderr" ]]; then
  echo "naming lint rejected valid target spellings" >&2
  exit 1
fi

run_stage0 lint "$project_fixture" \
  >"$output_root/project-stage0.stdout" \
  2>"$output_root/project-stage0.stderr"
run_self lint "$project_fixture" \
  >"$output_root/project-self.stdout" \
  2>"$output_root/project-self.stderr"
cmp "$output_root/project-stage0.stderr" "$output_root/project-self.stderr"
if ! grep -Fq "warning[MLG-NAME-009]" "$output_root/project-stage0.stderr"; then
  echo "naming lint did not classify the project name" >&2
  exit 1
fi

echo "naming lint checks passed"
