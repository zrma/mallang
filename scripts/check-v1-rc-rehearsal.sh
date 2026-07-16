#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

reuse_release_artifact=0
if [[ $# -gt 1 ]] || [[ $# -eq 1 && "$1" != "--reuse-release-artifact" ]]; then
  echo "usage: scripts/check-v1-rc-rehearsal.sh [--reuse-release-artifact]" >&2
  exit 2
fi
if [[ $# -eq 1 ]]; then
  reuse_release_artifact=1
fi

rc_version="$(sed -n '/^\[package\]/,/^\[/ s/^version = "\([^"]*\)"/\1/p' Cargo.toml | head -n 1)"
if [[ "$rc_version" != "1.0.0-rc.1" ]]; then
  echo "v1 RC rehearsal requires Cargo version 1.0.0-rc.1, got: $rc_version" >&2
  exit 1
fi

artifact_work="target/mallang/release-artifact-smoke"
if [[ "$reuse_release_artifact" -eq 0 ]]; then
  rc_archive="$(scripts/check-release-artifacts.sh)"
else
  shopt -s nullglob
  rc_archives=("$artifact_work/first/mallang-v${rc_version}-"*.tar.gz)
  shopt -u nullglob
  if [[ "${#rc_archives[@]}" -ne 1 ]]; then
    echo "v1 RC rehearsal expected one reusable host archive" >&2
    exit 1
  fi
  rc_archive="${rc_archives[0]}"
fi

rc_checksums="$artifact_work/offline/SHA256SUMS"
clean_installed="$artifact_work/home/.local/bin/mlg"
if [[ ! -f "$rc_archive" || ! -f "$rc_checksums" || ! -x "$clean_installed" ]] || \
  [[ "$($clean_installed --version)" != "mlg $rc_version" ]]; then
  echo "v1 RC rehearsal reusable clean-install evidence is incomplete" >&2
  exit 1
fi

work="target/mallang/v1-rc-rehearsal"
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
    echo "v1 RC rehearsal $label online installation mismatch" >&2
    exit 1
  fi
}

install_rc() {
  local label="$1"
  ./install.sh \
    --version "$rc_version" \
    --bin-dir "$prefix/bin" \
    --archive "$rc_archive" \
    --checksums "$rc_checksums" \
    >"$work/$label-install.stdout" 2>"$work/$label-install.stderr"
  if [[ "$(cat "$work/$label-install.stdout")" != "installed mlg $rc_version to $mlg" ]] || \
    [[ -s "$work/$label-install.stderr" ]] || [[ "$($mlg --version)" != "mlg $rc_version" ]]; then
    echo "v1 RC rehearsal $label offline installation mismatch" >&2
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
    echo "v1 RC rehearsal $label dogfood emitted stderr" >&2
    exit 1
  fi
}

install_online "v09" "0.9.0"
exercise "v09" "0.9.0"
install_rc "upgrade"
exercise "upgrade" "$rc_version"
install_online "rollback" "0.9.0"
exercise "rollback" "0.9.0"
install_rc "reupgrade"
exercise "reupgrade" "$rc_version"

for label in upgrade rollback reupgrade; do
  for relative in \
    iteration-1-binary.stdout \
    iteration-1-run.stdout \
    iteration-1-test.stdout \
    iteration-1-usage.stderr \
    summary-1.txt; do
    if ! cmp -s "$work/dogfood-v09/$relative" "$work/dogfood-$label/$relative"; then
      echo "v1 RC rehearsal observable mismatch: $label/$relative" >&2
      exit 1
    fi
  done
done

if [[ "$($mlg --version)" != "mlg $rc_version" ]]; then
  echo "v1 RC rehearsal did not finish on the release candidate" >&2
  exit 1
fi

echo "v1 RC clean install, v0.9 upgrade, rollback, re-upgrade, and textstats rehearsal passed"
