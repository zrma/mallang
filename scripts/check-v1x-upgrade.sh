#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

reuse_release_artifact=0
if [[ $# -gt 1 ]] || [[ $# -eq 1 && "$1" != "--reuse-release-artifact" ]]; then
  echo "usage: scripts/check-v1x-upgrade.sh [--reuse-release-artifact]" >&2
  exit 2
fi
if [[ $# -eq 1 ]]; then
  reuse_release_artifact=1
fi

base_version="1.0.0"
current_version="$(
  sed -n '/^\[package\]/,/^\[/ s/^version = "\([^"]*\)"/\1/p' Cargo.toml | head -n 1
)"
if [[ ! "$current_version" =~ ^1\.[1-9][0-9]*\.[0-9]+$ ]]; then
  echo "v1.x upgrade rehearsal requires a post-1.0 stable version, got: $current_version" >&2
  exit 1
fi

artifact_work="target/mallang/release-artifact-smoke"
if [[ "$reuse_release_artifact" -eq 0 ]]; then
  current_archive="$(scripts/check-release-artifacts.sh)"
else
  shopt -s nullglob
  current_archives=("$artifact_work/first/mallang-v${current_version}-"*.tar.gz)
  shopt -u nullglob
  if [[ "${#current_archives[@]}" -ne 1 ]]; then
    echo "v1.x upgrade rehearsal expected one reusable host archive" >&2
    exit 1
  fi
  current_archive="${current_archives[0]}"
fi

current_checksums="$artifact_work/offline/SHA256SUMS"
clean_installed="$artifact_work/home/.local/bin/mlg"
if [[ ! -f "$current_archive" || ! -f "$current_checksums" || ! -x "$clean_installed" ]] || \
  [[ "$($clean_installed --version)" != "mlg $current_version" ]]; then
  echo "v1.x upgrade rehearsal reusable clean-install evidence is incomplete" >&2
  exit 1
fi

work="target/mallang/v1x-upgrade"
prefix="$work/prefix"
mlg="$prefix/bin/mlg"
rm -rf "$work"
mkdir -p "$prefix/bin"

install_base() {
  local label="$1"
  ./install.sh --version "$base_version" --bin-dir "$prefix/bin" \
    >"$work/$label-install.stdout" 2>"$work/$label-install.stderr"
  if [[ "$(cat "$work/$label-install.stdout")" != "installed mlg $base_version to $mlg" ]] || \
    [[ -s "$work/$label-install.stderr" ]] || [[ "$($mlg --version)" != "mlg $base_version" ]]; then
    echo "v1.x upgrade rehearsal $label base installation mismatch" >&2
    exit 1
  fi
}

install_current() {
  local label="$1"
  ./install.sh \
    --version "$current_version" \
    --bin-dir "$prefix/bin" \
    --archive "$current_archive" \
    --checksums "$current_checksums" \
    >"$work/$label-install.stdout" 2>"$work/$label-install.stderr"
  if [[ "$(cat "$work/$label-install.stdout")" != "installed mlg $current_version to $mlg" ]] || \
    [[ -s "$work/$label-install.stderr" ]] || [[ "$($mlg --version)" != "mlg $current_version" ]]; then
    echo "v1.x upgrade rehearsal $label current installation mismatch" >&2
    exit 1
  fi
}

exercise_v1_source() {
  local label="$1"
  local version="$2"
  scripts/check-v09-dogfood.sh \
    --compiler "$mlg" \
    --expected-version "$version" \
    --work-dir "$work/dogfood-$label" \
    >"$work/$label-dogfood.stdout" 2>"$work/$label-dogfood.stderr"
  if [[ -s "$work/$label-dogfood.stderr" ]]; then
    echo "v1.x upgrade rehearsal $label dogfood emitted stderr" >&2
    exit 1
  fi
}

exercise_streaming_addition() {
  local label="$1"
  local input="$work/$label-streaming-input.txt"
  printf 'INFO first\nERROR 둘\nERROR final' >"$input"
  local output
  output="$(
    "$mlg" run tests/fixtures/v11-streaming-io/for-each-line.mlg -- "$input" ERROR
  )"
  if [[ "$output" != $'2\nERROR 둘\n3\nERROR final\n2' ]]; then
    echo "v1.x upgrade rehearsal $label streaming output mismatch" >&2
    exit 1
  fi
}

install_base "base"
exercise_v1_source "base" "$base_version"
install_current "upgrade"
exercise_v1_source "upgrade" "$current_version"
exercise_streaming_addition "upgrade"
install_base "rollback"
exercise_v1_source "rollback" "$base_version"
install_current "reupgrade"
exercise_v1_source "reupgrade" "$current_version"
exercise_streaming_addition "reupgrade"

for label in upgrade rollback reupgrade; do
  for relative in \
    iteration-1-binary.stdout \
    iteration-1-run.stdout \
    iteration-1-test.stdout \
    iteration-1-usage.stderr \
    summary-1.txt; do
    if ! cmp -s "$work/dogfood-base/$relative" "$work/dogfood-$label/$relative"; then
      echo "v1.x upgrade rehearsal observable mismatch: $label/$relative" >&2
      exit 1
    fi
  done
done

if [[ "$($mlg --version)" != "mlg $current_version" ]]; then
  echo "v1.x upgrade rehearsal did not finish on the current release" >&2
  exit 1
fi

echo "v1.0 upgrade, rollback, v1.x re-upgrade, compatibility, and streaming rehearsal passed"
