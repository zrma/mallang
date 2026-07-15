#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

deep_sanitizers=1
if [[ $# -gt 1 ]]; then
  echo "usage: scripts/check-v08-acceptance.sh [--skip-deep-sanitizers]" >&2
  exit 2
fi
if [[ $# -eq 1 ]]; then
  if [[ "$1" != "--skip-deep-sanitizers" ]]; then
    echo "usage: scripts/check-v08-acceptance.sh [--skip-deep-sanitizers]" >&2
    exit 2
  fi
  deep_sanitizers=0
fi

scripts/check.sh
scripts/check-release-binary.sh
if [[ "$deep_sanitizers" -eq 1 ]]; then
  scripts/check-generated-c-sanitizers.sh --assume-generated
fi

echo "v0.8 compiler hardening acceptance passed"
