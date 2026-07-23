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
  ! grep -Fq -- '--jobs <count>' "$work/self-hosting-help.stderr" || \
  ! grep -Fq -- '--compiler-pair <stage1> <stage2>' \
    "$work/self-hosting-help.stderr"; then
  echo "self-hosting gate help is missing focused or worker controls" >&2
  exit 1
fi

expect_status_2 invalid-focus scripts/check-self-hosting-lexer.sh --focus nope
grep -Fq 'unknown self-hosting focus area: nope' "$work/invalid-focus.stderr"

expect_status_2 invalid-jobs scripts/check-self-hosting-lexer.sh --jobs 0
grep -Fq 'self-hosting jobs must be a positive integer' "$work/invalid-jobs.stderr"

expect_status_2 missing-compiler-pair \
  scripts/check-self-hosting-lexer.sh --compiler-pair stage1
grep -Fq 'usage: scripts/check-self-hosting-lexer.sh' \
  "$work/missing-compiler-pair.stderr"

expect_status_2 missing-worker-args scripts/check-self-hosting-fixture.sh
grep -Fq 'usage: scripts/check-self-hosting-fixture.sh' "$work/missing-worker-args.stderr"

scripts/diagnose-self-hosting-compiler-ir.sh --help \
  >"$work/compiler-ir-help.stdout" 2>"$work/compiler-ir-help.stderr"
grep -Fq -- '--rebuild-bootstrap' "$work/compiler-ir-help.stderr"
grep -Fq -- '--reuse-bootstrap' "$work/compiler-ir-help.stderr"

scripts/check-self-hosting-backend.sh --help \
  >"$work/backend-help.stdout" 2>"$work/backend-help.stderr"
grep -Fq -- '--assume-bootstrap' "$work/backend-help.stderr"
grep -Fq -- '--fixtures-only' "$work/backend-help.stderr"
expect_status_2 invalid-backend-mode scripts/check-self-hosting-backend.sh --fast
grep -Fq 'usage: scripts/check-self-hosting-backend.sh' \
  "$work/invalid-backend-mode.stderr"

scripts/check-self-hosting-fixed-point.sh --help \
  >"$work/fixed-point-help.stdout" 2>"$work/fixed-point-help.stderr"
grep -Fq -- '--assume-bootstrap' "$work/fixed-point-help.stderr"
grep -Fq -- '--skip-sanitizers' "$work/fixed-point-help.stderr"
grep -Fq -- '--jobs <count>' "$work/fixed-point-help.stderr"
expect_status_2 invalid-fixed-point-mode \
  scripts/check-self-hosting-fixed-point.sh --fast
grep -Fq 'usage: scripts/check-self-hosting-fixed-point.sh' \
  "$work/invalid-fixed-point-mode.stderr"

scripts/build-self-hosted-compiler.sh --help \
  >"$work/self-compiler-build-help.stdout" \
  2>"$work/self-compiler-build-help.stderr"
grep -Fq -- '--stage0 <path>' "$work/self-compiler-build-help.stderr"
grep -Fq -- '--output <path>' "$work/self-compiler-build-help.stderr"
expect_status_2 invalid-default-compiler-mode \
  scripts/check-self-hosting-default-compiler.sh --fast
grep -Fq 'usage: scripts/check-self-hosting-default-compiler.sh' \
  "$work/invalid-default-compiler-mode.stderr"

cat >"$work/ir-expected.txt" <<'EOF'
IR|2
FUNCTION|first|unit|0|1
I|0|S|Stmt.Return|0|0|0||unit|0
FUNCTION|second|unit|0|1
I|0|S|Stmt.Return|0|0|0||unit|0
EOF
cp "$work/ir-expected.txt" "$work/ir-actual.txt"
scripts/compare-self-hosting-ir.py \
  "$work/ir-expected.txt" "$work/ir-actual.txt" \
  >"$work/ir-match.stdout" 2>"$work/ir-match.stderr"
grep -Fq 'matching=2 mismatching=0' "$work/ir-match.stdout"

sed 's/FUNCTION|second|unit|0|1/FUNCTION|second|unit|0|2/' \
  "$work/ir-expected.txt" >"$work/ir-actual.txt"
if scripts/compare-self-hosting-ir.py \
  --max-diff-lines 8 \
  "$work/ir-expected.txt" "$work/ir-actual.txt" \
  >"$work/ir-mismatch.stdout" 2>"$work/ir-mismatch.stderr"; then
  echo "self-hosting IR mismatch comparison unexpectedly succeeded" >&2
  exit 1
fi
grep -Fq 'matching=1 mismatching=1' "$work/ir-mismatch.stdout"
grep -Fq 'first mismatching function: second' "$work/ir-mismatch.stderr"

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

grep -Fq 'scripts/check-self-hosting-backend.sh --assume-bootstrap' scripts/check.sh
grep -Fq 'scripts/check-self-hosting-fixed-point.sh' .github/workflows/ci.yml
grep -Fq 'scripts/check-self-hosting-default-compiler.sh' .github/workflows/ci.yml

echo "self-hosting gate interface and CI role separation passed"
