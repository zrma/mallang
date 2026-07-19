#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

work="$(mktemp -d "${TMPDIR:-/tmp}/mallang-self-hosting-interface.XXXXXX")"
trap 'rm -rf "$work"' EXIT

expect_status_2() {
  local label="$1"
  shift
  if "$@" >"$work/$label.stdout" 2>"$work/$label.stderr"; then
    echo "$label unexpectedly succeeded" >&2
    exit 1
  else
    status=$?
  fi
  if [[ "$status" -ne 2 ]]; then
    echo "$label returned $status instead of 2" >&2
    cat "$work/$label.stderr" >&2
    exit 1
  fi
}

scripts/check-self-hosting-lexer.sh --help \
  >"$work/self-hosting-help.stdout" 2>"$work/self-hosting-help.stderr"
if ! grep -Fq -- '--focus <area>' "$work/self-hosting-help.stderr" || \
  ! grep -Fq -- '--jobs <count>' "$work/self-hosting-help.stderr"; then
  echo "self-hosting gate help is missing focused or worker controls" >&2
  exit 1
fi

expect_status_2 invalid-focus scripts/check-self-hosting-lexer.sh --focus nope
grep -Fq 'unknown self-hosting focus area: nope' "$work/invalid-focus.stderr"

expect_status_2 invalid-jobs scripts/check-self-hosting-lexer.sh --jobs 0
grep -Fq 'self-hosting jobs must be a positive integer' "$work/invalid-jobs.stderr"

expect_status_2 missing-worker-args scripts/check-self-hosting-fixture.sh
grep -Fq 'usage: scripts/check-self-hosting-fixture.sh' "$work/missing-worker-args.stderr"

scripts/check-v08-acceptance.sh --help \
  >"$work/v08-help.stdout" 2>"$work/v08-help.stderr"
scripts/check-v1x-acceptance.sh --help \
  >"$work/v1x-help.stdout" 2>"$work/v1x-help.stderr"
for help_file in "$work/v08-help.stderr" "$work/v1x-help.stderr"; do
  grep -Fq -- '--skip-core-check' "$help_file"
  grep -Fq -- '--skip-deep-sanitizers' "$help_file"
done

expect_status_2 unsafe-v08-skip scripts/check-v08-acceptance.sh --skip-core-check
grep -Fq -- '--skip-core-check requires --skip-deep-sanitizers' \
  "$work/unsafe-v08-skip.stderr"

if [[ "$(grep -Fc 'scripts/check.sh' .github/workflows/ci.yml)" -ne 1 ]] || \
  ! grep -Fq 'scripts/check-generated-c-sanitizers.sh --assume-generated' \
    .github/workflows/ci.yml || \
  ! grep -Fq \
    'scripts/check-v1x-acceptance.sh --skip-core-check --skip-deep-sanitizers' \
    .github/workflows/ci.yml; then
  echo "CI must run one canonical core check and platform-only release acceptance" >&2
  exit 1
fi

echo "self-hosting gate interface and CI role separation passed"
