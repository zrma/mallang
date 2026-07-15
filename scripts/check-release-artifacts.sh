#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

command -v clang >/dev/null 2>&1 || {
  echo "clang is required for release artifact smoke" >&2
  exit 1
}

crate_version="$(
  sed -n '/^\[package\]/,/^\[/ s/^version = "\([^"]*\)"/\1/p' Cargo.toml | head -n 1
)"
work="target/mallang/release-artifact-smoke"
first="$work/first"
second="$work/second"
offline="$work/offline"
home="$work/home"
explicit_prefix="$work/explicit"
project="$work/project"
rm -rf "$work"
mkdir -p "$first" "$second" "$offline" "$project/src" "$project/tests"

first_archive="$(
  scripts/build-release-artifact.sh --version "$crate_version" --output-dir "$first"
)"
second_archive="$(
  scripts/build-release-artifact.sh --version "$crate_version" --output-dir "$second"
)"
if ! cmp -s "$first_archive" "$second_archive"; then
  echo "repeated release archive builds are not byte-identical" >&2
  exit 1
fi

archive_name="$(basename "$first_archive")"
cp "$first_archive" "$offline/$archive_name"
python3 scripts/write-release-checksums.py \
  --output "$offline/SHA256SUMS" \
  "$offline/$archive_name"

if python3 scripts/write-release-checksums.py \
  --require-all-targets \
  --output "$work/incomplete/SHA256SUMS" \
  "$offline/$archive_name" >"$work/incomplete.stdout" 2>"$work/incomplete.stderr"; then
  echo "incomplete release target checksum set unexpectedly succeeded" >&2
  exit 1
fi
if ! grep -Fq "release target set mismatch" "$work/incomplete.stderr"; then
  echo "incomplete release target checksum diagnostic mismatch" >&2
  cat "$work/incomplete.stderr" >&2
  exit 1
fi

mkdir -p "$work/combined"
cp "$offline/$archive_name" "$work/combined/$archive_name"
case "$archive_name" in
  *-aarch64-apple-darwin.tar.gz)
    other_archive="mallang-v${crate_version}-x86_64-unknown-linux-gnu.tar.gz"
    ;;
  *-x86_64-unknown-linux-gnu.tar.gz)
    other_archive="mallang-v${crate_version}-aarch64-apple-darwin.tar.gz"
    ;;
  *)
    echo "unexpected native release archive name: $archive_name" >&2
    exit 1
    ;;
esac
cp "$offline/$archive_name" "$work/combined/$other_archive"
python3 scripts/write-release-checksums.py \
  --require-all-targets \
  --output "$work/combined/SHA256SUMS" \
  "$work/combined"/*.tar.gz
if [[ "$(wc -l <"$work/combined/SHA256SUMS" | tr -d ' ')" != "2" ]] || \
   ! LC_ALL=C sort -c -k2,2 "$work/combined/SHA256SUMS"; then
  echo "combined release checksum ordering mismatch" >&2
  exit 1
fi

mkdir -p "$work/tampered"
cp "$offline/$archive_name" "$work/tampered/$archive_name"
printf 'tampered' >>"$work/tampered/$archive_name"
if ./install.sh \
  --version "$crate_version" \
  --bin-dir "$work/tampered-bin" \
  --archive "$work/tampered/$archive_name" \
  --checksums "$offline/SHA256SUMS" \
  >"$work/tampered.stdout" 2>"$work/tampered.stderr"; then
  echo "tampered release archive unexpectedly installed" >&2
  exit 1
fi
if [[ -s "$work/tampered.stdout" ]] || \
   [[ "$(cat "$work/tampered.stderr")" != "checksum mismatch for $archive_name" ]]; then
  echo "tampered release archive diagnostic mismatch" >&2
  cat "$work/tampered.stdout" >&2
  cat "$work/tampered.stderr" >&2
  exit 1
fi

mkdir -p "$work/malformed"
python3 - "$offline/$archive_name" "$work/malformed/$archive_name" <<'PY'
import gzip
import io
from pathlib import Path
import sys
import tarfile

source_path = Path(sys.argv[1])
output_path = Path(sys.argv[2])
with tarfile.open(source_path, "r:gz") as source:
    members = source.getmembers()
    root_name = members[0].name.rstrip("/")
    with output_path.open("wb") as raw:
        with gzip.GzipFile(filename="", mode="wb", fileobj=raw, mtime=0) as zipped:
            with tarfile.open(fileobj=zipped, mode="w", format=tarfile.USTAR_FORMAT) as output:
                for member in members:
                    extracted = source.extractfile(member) if member.isfile() else None
                    output.addfile(member, extracted)
                extra = tarfile.TarInfo(f"{root_name}/EXTRA")
                extra.mode = 0o644
                extra.size = 5
                extra.mtime = 0
                extra.uid = 0
                extra.gid = 0
                extra.uname = "root"
                extra.gname = "root"
                output.addfile(extra, io.BytesIO(b"extra"))
PY
python3 scripts/write-release-checksums.py \
  --output "$work/malformed/SHA256SUMS" \
  "$work/malformed/$archive_name"
if ./install.sh \
  --version "$crate_version" \
  --bin-dir "$work/malformed-bin" \
  --archive "$work/malformed/$archive_name" \
  --checksums "$work/malformed/SHA256SUMS" \
  >"$work/malformed.stdout" 2>"$work/malformed.stderr"; then
  echo "malformed release archive unexpectedly installed" >&2
  exit 1
fi
if [[ -s "$work/malformed.stdout" ]] || \
   [[ "$(cat "$work/malformed.stderr")" != "archive entry set mismatch for $archive_name" ]]; then
  echo "malformed release archive diagnostic mismatch" >&2
  cat "$work/malformed.stdout" >&2
  cat "$work/malformed.stderr" >&2
  exit 1
fi

mkdir -p "$work/symlink"
python3 - "$offline/$archive_name" "$work/symlink/$archive_name" <<'PY'
import gzip
from pathlib import Path
import sys
import tarfile

source_path = Path(sys.argv[1])
output_path = Path(sys.argv[2])
with tarfile.open(source_path, "r:gz") as source:
    members = source.getmembers()
    root_name = members[0].name.rstrip("/")
    with output_path.open("wb") as raw:
        with gzip.GzipFile(filename="", mode="wb", fileobj=raw, mtime=0) as zipped:
            with tarfile.open(fileobj=zipped, mode="w", format=tarfile.USTAR_FORMAT) as output:
                for member in members:
                    if member.name == f"{root_name}/bin":
                        member.type = tarfile.SYMTYPE
                        member.linkname = "/tmp/mallang-release-escape"
                        member.size = 0
                        output.addfile(member)
                    else:
                        extracted = source.extractfile(member) if member.isfile() else None
                        output.addfile(member, extracted)
PY
python3 scripts/write-release-checksums.py \
  --output "$work/symlink/SHA256SUMS" \
  "$work/symlink/$archive_name"
if ./install.sh \
  --version "$crate_version" \
  --bin-dir "$work/symlink-bin" \
  --archive "$work/symlink/$archive_name" \
  --checksums "$work/symlink/SHA256SUMS" \
  >"$work/symlink.stdout" 2>"$work/symlink.stderr"; then
  echo "symlink release archive unexpectedly installed" >&2
  exit 1
fi
if [[ -s "$work/symlink.stdout" ]] || \
   [[ "$(cat "$work/symlink.stderr")" != "archive entry set mismatch for $archive_name" ]]; then
  echo "symlink release archive diagnostic mismatch" >&2
  cat "$work/symlink.stdout" >&2
  cat "$work/symlink.stderr" >&2
  exit 1
fi

install_args=(
  --version "$crate_version"
  --archive "$offline/$archive_name"
  --checksums "$offline/SHA256SUMS"
)
HOME="$home" ./install.sh "${install_args[@]}" >"$work/install-default.stdout"
HOME="$home" ./install.sh "${install_args[@]}" >"$work/reinstall-default.stdout"
installed="$home/.local/bin/mlg"
if [[ "$(cat "$work/install-default.stdout")" != "installed mlg $crate_version to $installed" ]] || \
   [[ "$(cat "$work/reinstall-default.stdout")" != "installed mlg $crate_version to $installed" ]]; then
  echo "default install/reinstall output mismatch" >&2
  exit 1
fi

./install.sh \
  --version "$crate_version" \
  --bin-dir "$explicit_prefix/bin" \
  --archive "$offline/$archive_name" \
  --checksums "$offline/SHA256SUMS" \
  >"$work/install-explicit.stdout"
if [[ ! -x "$explicit_prefix/bin/mlg" ]]; then
  echo "explicit-prefix install did not produce mlg" >&2
  exit 1
fi

if [[ "$($installed --version)" != "mlg $crate_version" ]]; then
  echo "installed release binary version mismatch" >&2
  exit 1
fi
help_output="$($installed --help)"
if [[ "$help_output" != *"usage:"* || "$help_output" != *"$installed test <input>"* ]]; then
  echo "installed release binary help mismatch" >&2
  exit 1
fi

cp examples/projects/hello/mallang.toml "$project/"
cp -R examples/projects/hello/src/. "$project/src/"
cp -R examples/projects/hello/tests/. "$project/tests/"

"$installed" check "$project" >"$work/project-check.stdout" 2>"$work/project-check.stderr"
"$installed" build "$project" -o "$work/hello" >"$work/project-build.stdout" 2>"$work/project-build.stderr"
if [[ "$(cat "$work/project-check.stdout")" != "$project: ok" ]] || \
   [[ "$(cat "$work/project-build.stdout")" != "$work/hello" ]] || \
   [[ -s "$work/project-check.stderr" || -s "$work/project-build.stderr" ]] || \
   [[ ! -x "$work/hello" ]]; then
  echo "installed release binary project check/build failed" >&2
  cat "$work/project-check.stderr" >&2
  cat "$work/project-build.stderr" >&2
  exit 1
fi
if [[ "$($work/hello)" != $'kim\n42\n22\n15\ngeneric\n8\nupdated\n13\n0' ]]; then
  echo "installed release binary built program output mismatch" >&2
  exit 1
fi

run_output="$($installed run "$project")"
if [[ "$run_output" != $'kim\n42\n22\n15\ngeneric\n8\nupdated\n13\n0' ]]; then
  echo "installed release binary project run output mismatch" >&2
  exit 1
fi
test_output="$($installed test "$project")"
expected_test_output=$'test hello/greet::ReadsPrivateProductionState ... ok\ntest hello::CopyAndOwnedValues ... ok\ntest hello::GenericAndClosure ... ok\ntest hello::MapAndStandardIo ... ok\ntest hello::RecursiveAdt ... ok\ntest result: ok. 5 passed; 0 failed'
if [[ "$test_output" != "$expected_test_output" ]]; then
  echo "installed release binary project test output mismatch" >&2
  printf '%s\n' "$test_output" >&2
  exit 1
fi

printf '%s\n' "$first_archive"
