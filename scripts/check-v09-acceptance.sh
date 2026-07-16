#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

if [[ $# -gt 1 ]] || [[ $# -eq 1 && "$1" != "--skip-deep-sanitizers" ]]; then
  echo "usage: scripts/check-v09-acceptance.sh [--skip-deep-sanitizers]" >&2
  exit 2
fi

python3 scripts/check-v09-freeze.py
python3 scripts/check-v1-conformance.py
scripts/check-v08-acceptance.sh "$@"

echo "v0.9 language freeze acceptance passed"
