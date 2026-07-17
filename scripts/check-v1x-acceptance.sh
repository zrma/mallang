#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

if [[ $# -gt 1 ]] || [[ $# -eq 1 && "$1" != "--skip-deep-sanitizers" ]]; then
  echo "usage: scripts/check-v1x-acceptance.sh [--skip-deep-sanitizers]" >&2
  exit 2
fi

crate_version="$(
  sed -n '/^\[package\]/,/^\[/ s/^version = "\([^"]*\)"/\1/p' Cargo.toml | head -n 1
)"
if [[ ! "$crate_version" =~ ^1\.[0-9]+\.[0-9]+$ ]]; then
  echo "post-stable acceptance requires a stable 1.x version, got: $crate_version" >&2
  exit 1
fi

python3 scripts/check-v1-conformance.py
scripts/check-v08-acceptance.sh "$@"
scripts/check-release-artifacts.sh >/dev/null
scripts/check-v1x-upgrade.sh --reuse-release-artifact

echo "Mallang $crate_version post-stable compatibility, runtime, and release artifact acceptance passed"
