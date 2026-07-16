#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

reuse_release_artifact=0
if [[ $# -gt 1 ]] || [[ $# -eq 1 && "$1" != "--reuse-release-artifact" ]]; then
  echo "usage: scripts/check-v1-stable-rehearsal.sh [--reuse-release-artifact]" >&2
  exit 2
fi
if [[ $# -eq 1 ]]; then
  reuse_release_artifact=1
fi

stable_version="$(sed -n '/^\[package\]/,/^\[/ s/^version = "\([^"]*\)"/\1/p' Cargo.toml | head -n 1)"
if [[ "$stable_version" != "1.0.0" ]]; then
  echo "v1 stable rehearsal requires Cargo version 1.0.0, got: $stable_version" >&2
  exit 1
fi

artifact_work="target/mallang/release-artifact-smoke"
if [[ "$reuse_release_artifact" -eq 0 ]]; then
  stable_archive="$(scripts/check-release-artifacts.sh)"
else
  shopt -s nullglob
  stable_archives=("$artifact_work/first/mallang-v${stable_version}-"*.tar.gz)
  shopt -u nullglob
  if [[ "${#stable_archives[@]}" -ne 1 ]]; then
    echo "v1 stable rehearsal expected one reusable host archive" >&2
    exit 1
  fi
  stable_archive="${stable_archives[0]}"
fi

stable_checksums="$artifact_work/offline/SHA256SUMS"
clean_installed="$artifact_work/home/.local/bin/mlg"
if [[ ! -f "$stable_archive" || ! -f "$stable_checksums" || ! -x "$clean_installed" ]] || \
  [[ "$($clean_installed --version)" != "mlg $stable_version" ]]; then
  echo "v1 stable rehearsal reusable clean-install evidence is incomplete" >&2
  exit 1
fi

work="target/mallang/v1-stable-rehearsal"
prefix="$work/prefix"
mlg="$prefix/bin/mlg"
rm -rf "$work"
mkdir -p "$prefix/bin"

install_online() {
  local label="$1"
  local version="$2"
  ./install.sh --version "$version" --bin-dir "$prefix/bin" \
    >"$work/$label-install.stdout" 2>"$work/$label-install.stderr"
  if [[ "$(cat "$work/$label-install.stdout")" != "installed mlg $version to $mlg" ]] || \
    [[ -s "$work/$label-install.stderr" ]] || [[ "$($mlg --version)" != "mlg $version" ]]; then
    echo "v1 stable rehearsal $label online installation mismatch" >&2
    exit 1
  fi
}

install_stable() {
  local label="$1"
  ./install.sh \
    --version "$stable_version" \
    --bin-dir "$prefix/bin" \
    --archive "$stable_archive" \
    --checksums "$stable_checksums" \
    >"$work/$label-install.stdout" 2>"$work/$label-install.stderr"
  if [[ "$(cat "$work/$label-install.stdout")" != "installed mlg $stable_version to $mlg" ]] || \
    [[ -s "$work/$label-install.stderr" ]] || [[ "$($mlg --version)" != "mlg $stable_version" ]]; then
    echo "v1 stable rehearsal $label offline installation mismatch" >&2
    exit 1
  fi
}

exercise() {
  local label="$1"
  local version="$2"
  scripts/check-v09-dogfood.sh \
    --compiler "$mlg" \
    --expected-version "$version" \
    --work-dir "$work/dogfood-$label" \
    >"$work/$label-dogfood.stdout" 2>"$work/$label-dogfood.stderr"
  if [[ -s "$work/$label-dogfood.stderr" ]]; then
    echo "v1 stable rehearsal $label dogfood emitted stderr" >&2
    exit 1
  fi
}

install_online "rc" "1.0.0-rc.1"
exercise "rc" "1.0.0-rc.1"
install_stable "upgrade"
exercise "upgrade" "$stable_version"
install_online "rollback" "1.0.0-rc.1"
exercise "rollback" "1.0.0-rc.1"
install_stable "reupgrade"
exercise "reupgrade" "$stable_version"

for label in upgrade rollback reupgrade; do
  for relative in \
    iteration-1-binary.stdout \
    iteration-1-run.stdout \
    iteration-1-test.stdout \
    iteration-1-usage.stderr \
    summary-1.txt; do
    if ! cmp -s "$work/dogfood-rc/$relative" "$work/dogfood-$label/$relative"; then
      echo "v1 stable rehearsal observable mismatch: $label/$relative" >&2
      exit 1
    fi
  done
done

if [[ "$($mlg --version)" != "mlg $stable_version" ]]; then
  echo "v1 stable rehearsal did not finish on the stable release" >&2
  exit 1
fi

echo "v1 stable clean install, RC upgrade, rollback, re-upgrade, and textstats rehearsal passed"
