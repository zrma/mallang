#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

if [[ $# -gt 1 ]] || [[ $# -eq 1 && "$1" != "--skip-deep-sanitizers" ]]; then
  echo "usage: scripts/check-v1-rc-acceptance.sh [--skip-deep-sanitizers]" >&2
  exit 2
fi

scripts/check-v09-acceptance.sh "$@"
scripts/check-v1-rc-rehearsal.sh --reuse-release-artifact

echo "v1.0.0-rc.1 acceptance and rollback rehearsal passed"
