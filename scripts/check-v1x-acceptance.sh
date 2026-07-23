#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

usage() {
  cat >&2 <<'EOF'
usage: scripts/check-v1x-acceptance.sh [--skip-core-check] [--skip-deep-sanitizers]
EOF
}

v08_args=()
while [[ $# -gt 0 ]]; do
  case "$1" in
    --skip-core-check|--skip-deep-sanitizers)
      v08_args+=("$1")
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

crate_version="$(
  sed -n '/^\[package\]/,/^\[/ s/^version = "\([^"]*\)"/\1/p' Cargo.toml | head -n 1
)"
if [[ ! "$crate_version" =~ ^1\.[0-9]+\.[0-9]+$ ]]; then
  echo "post-stable acceptance requires a stable 1.x version, got: $crate_version" >&2
  exit 1
fi

python3 scripts/check-v1-conformance.py
if ((${#v08_args[@]} > 0)); then
  scripts/check-v08-acceptance.sh "${v08_args[@]}"
else
  scripts/check-v08-acceptance.sh
fi
scripts/check-release-artifacts.sh >/dev/null
if ! scripts/check-v1x-upgrade.sh --reuse-release-artifact; then
  echo "post-stable upgrade and rollback rehearsal failed" >&2
  exit 1
fi

echo "Mallang $crate_version post-stable compatibility, runtime, and release artifact acceptance passed"
