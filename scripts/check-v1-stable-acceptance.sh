#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

if [[ $# -gt 1 ]] || [[ $# -eq 1 && "$1" != "--skip-deep-sanitizers" ]]; then
  echo "usage: scripts/check-v1-stable-acceptance.sh [--skip-deep-sanitizers]" >&2
  exit 2
fi

git merge-base --is-ancestor v1.0.0-rc.1 HEAD
if ! git diff --quiet v0.9.0 -- src docs/conformance/v1-rules.json; then
  echo "v1 stable acceptance found a frozen compiler or conformance-map change after v0.9.0" >&2
  exit 1
fi

scripts/check-v09-acceptance.sh "$@"
scripts/check-v1-stable-rehearsal.sh --reuse-release-artifact

echo "v1.0.0 frozen-contract, stable upgrade, rollback, and release acceptance passed"
