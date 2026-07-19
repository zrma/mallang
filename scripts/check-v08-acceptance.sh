#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

usage() {
  cat >&2 <<'EOF'
usage: scripts/check-v08-acceptance.sh [--skip-core-check] [--skip-deep-sanitizers]
EOF
}

run_core_check=1
deep_sanitizers=1
while [[ $# -gt 0 ]]; do
  case "$1" in
    --skip-core-check)
      run_core_check=0
      ;;
    --skip-deep-sanitizers)
      deep_sanitizers=0
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      usage
      exit 2
      ;;
  esac
  shift
done

if [[ "$run_core_check" -eq 0 && "$deep_sanitizers" -eq 1 ]]; then
  echo "--skip-core-check requires --skip-deep-sanitizers" >&2
  exit 2
fi

if [[ "$run_core_check" -eq 1 ]]; then
  scripts/check.sh
fi
scripts/check-release-binary.sh
if [[ "$deep_sanitizers" -eq 1 ]]; then
  scripts/check-generated-c-sanitizers.sh --assume-generated
fi

echo "v0.8 compiler hardening acceptance passed"
