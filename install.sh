#!/usr/bin/env sh
set -eu

usage() {
  cat <<'EOF'
usage: ./install.sh --version <major.minor.patch[-prerelease]> [--bin-dir <directory>]
       ./install.sh --version <major.minor.patch[-prerelease]> [--bin-dir <directory>] --archive <path> --checksums <path>
EOF
}

fail_usage() {
  echo "$1" >&2
  usage >&2
  exit 2
}

version=""
bin_dir=""
archive_input=""
checksums_input=""

while [ "$#" -gt 0 ]; do
  case "$1" in
    --version)
      [ "$#" -ge 2 ] && [ -n "$2" ] || fail_usage "missing value for --version"
      version="$2"
      shift 2
      ;;
    --bin-dir)
      [ "$#" -ge 2 ] && [ -n "$2" ] || fail_usage "missing value for --bin-dir"
      bin_dir="$2"
      shift 2
      ;;
    --archive)
      [ "$#" -ge 2 ] && [ -n "$2" ] || fail_usage "missing value for --archive"
      archive_input="$2"
      shift 2
      ;;
    --checksums)
      [ "$#" -ge 2 ] && [ -n "$2" ] || fail_usage "missing value for --checksums"
      checksums_input="$2"
      shift 2
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      fail_usage "unknown argument: $1"
      ;;
  esac
done

case "$version" in
  ""|*[!0-9A-Za-z.-]*|.*|*.|*..*|-*|*-)
    fail_usage "--version must be major.minor.patch[-prerelease]"
    ;;
esac
version_core=${version%%-*}
if [ "$version_core" != "$version" ]; then
  version_prerelease=${version#"$version_core"-}
  case "$version_prerelease" in
    ""|.*|*.|*..*|*[!0-9A-Za-z.-]*)
      fail_usage "--version must be major.minor.patch[-prerelease]"
      ;;
  esac
fi
version_remainder=${version_core#*.}
version_patch=${version_remainder#*.}
version_major=${version_core%%.*}
version_minor=${version_remainder%%.*}
if [ "$version_remainder" = "$version_core" ] || \
   [ "$version_patch" = "$version_remainder" ] || \
   [ -z "$version_patch" ] || \
   [ "${version_patch#*.}" != "$version_patch" ] || \
   [ -z "$version_major" ] || \
   [ -z "$version_minor" ]; then
  fail_usage "--version must be major.minor.patch[-prerelease]"
fi
case "$version_major:$version_minor:$version_patch" in
  *[!0-9:]*|:*|*::*|*:)
    fail_usage "--version must be major.minor.patch[-prerelease]"
    ;;
esac

if [ -z "$bin_dir" ]; then
  [ -n "${HOME:-}" ] || {
    echo "HOME is required when --bin-dir is not specified" >&2
    exit 1
  }
  bin_dir="$HOME/.local/bin"
fi

if { [ -n "$archive_input" ] && [ -z "$checksums_input" ]; } || \
   { [ -z "$archive_input" ] && [ -n "$checksums_input" ]; }; then
  fail_usage "--archive and --checksums must be provided together"
fi

case "$(uname -s):$(uname -m)" in
  Darwin:arm64|Darwin:aarch64)
    target="aarch64-apple-darwin"
    ;;
  Linux:x86_64|Linux:amd64)
    target="x86_64-unknown-linux-gnu"
    ;;
  *)
    echo "unsupported install host: $(uname -s) $(uname -m)" >&2
    exit 1
    ;;
esac

for command_name in awk chmod cmp cp mktemp mv sort tar; do
  command -v "$command_name" >/dev/null 2>&1 || {
    echo "$command_name is required to install Mallang" >&2
    exit 1
  }
done
command -v clang >/dev/null 2>&1 || {
  echo "clang is required by mlg build, run, and test" >&2
  exit 1
}

if command -v sha256sum >/dev/null 2>&1; then
  sha256_file() {
    sha256sum "$1" | awk '{print $1}'
  }
elif command -v shasum >/dev/null 2>&1; then
  sha256_file() {
    shasum -a 256 "$1" | awk '{print $1}'
  }
else
  echo "sha256sum or shasum is required to install Mallang" >&2
  exit 1
fi

archive_name="mallang-v${version}-${target}.tar.gz"
root_name="mallang-v${version}-${target}"
temporary="$(mktemp -d "${TMPDIR:-/tmp}/mallang-install.XXXXXX")"
staged_driver=""
staged_compiler=""
cleanup() {
  [ -z "$staged_driver" ] || rm -f "$staged_driver"
  [ -z "$staged_compiler" ] || rm -f "$staged_compiler"
  rm -rf "$temporary"
}
trap cleanup EXIT HUP INT TERM

archive="$temporary/$archive_name"
checksums="$temporary/SHA256SUMS"
if [ -n "$archive_input" ]; then
  [ -f "$archive_input" ] && [ ! -L "$archive_input" ] || {
    echo "archive is not a regular file: $archive_input" >&2
    exit 1
  }
  [ -f "$checksums_input" ] && [ ! -L "$checksums_input" ] || {
    echo "checksums file is not a regular file: $checksums_input" >&2
    exit 1
  }
  cp "$archive_input" "$archive"
  cp "$checksums_input" "$checksums"
else
  command -v curl >/dev/null 2>&1 || {
    echo "curl is required to download Mallang release files" >&2
    exit 1
  }
  base_url="https://github.com/zrma/mallang/releases/download/v${version}"
  curl --fail --location --silent --show-error --proto '=https' --proto-redir '=https' \
    --tlsv1.2 "$base_url/$archive_name" --output "$archive"
  curl --fail --location --silent --show-error --proto '=https' --proto-redir '=https' \
    --tlsv1.2 "$base_url/SHA256SUMS" --output "$checksums"
fi

match_count="$(
  awk -v name="$archive_name" '
    NF == 2 && $2 == name && length($1) == 64 &&
      $1 !~ /[^0-9a-f]/ && $0 == $1 "  " $2 { count += 1 }
    END { print count + 0 }
  ' "$checksums"
)"
if [ "$match_count" -ne 1 ]; then
  echo "SHA256SUMS must contain exactly one valid entry for $archive_name" >&2
  exit 1
fi
expected_checksum="$(
  awk -v name="$archive_name" '
    NF == 2 && $2 == name && length($1) == 64 &&
      $1 !~ /[^0-9a-f]/ && $0 == $1 "  " $2 { print $1 }
  ' "$checksums"
)"
actual_checksum="$(sha256_file "$archive")"
if [ "$actual_checksum" != "$expected_checksum" ]; then
  echo "checksum mismatch for $archive_name" >&2
  exit 1
fi

actual_entries="$temporary/actual-entries"
expected_entries="$temporary/expected-entries"
tar -tvzf "$archive" | \
  awk 'NF { print substr($1, 1, 1) " " $NF }' | \
  LC_ALL=C sort >"$actual_entries"
cat >"$expected_entries" <<EOF
d $root_name/
- $root_name/LICENSE-APACHE
- $root_name/LICENSE-MIT
- $root_name/README.md
d $root_name/bin/
- $root_name/bin/mlg
- $root_name/bin/mlgc
EOF
LC_ALL=C sort -o "$expected_entries" "$expected_entries"
if ! cmp -s "$actual_entries" "$expected_entries"; then
  echo "archive entry set mismatch for $archive_name" >&2
  exit 1
fi

extract_dir="$temporary/extract"
mkdir -p "$extract_dir"
tar -xzf "$archive" -C "$extract_dir"
for relative in bin/mlg bin/mlgc LICENSE-MIT LICENSE-APACHE README.md; do
  extracted="$extract_dir/$root_name/$relative"
  [ -f "$extracted" ] && [ ! -L "$extracted" ] || {
    echo "archive contains an invalid file: $relative" >&2
    exit 1
  }
done

extracted_driver="$extract_dir/$root_name/bin/mlg"
extracted_compiler="$extract_dir/$root_name/bin/mlgc"
compiler_version="$($extracted_compiler --version)"
if [ "$compiler_version" != "mlgc protocol 1" ]; then
  echo "installed compiler protocol mismatch: expected mlgc protocol 1, got $compiler_version" >&2
  exit 1
fi
driver_version="$($extracted_driver --version)"
if [ "$driver_version" != "mlg $version" ]; then
  echo "installed driver version mismatch: expected mlg $version, got $driver_version" >&2
  exit 1
fi
stage0_version="$($extracted_driver --compiler stage0 --version)"
if [ "$stage0_version" != "mlg $version" ]; then
  echo "installed Stage0 recovery version mismatch: expected mlg $version, got $stage0_version" >&2
  exit 1
fi

mkdir -p "$bin_dir"
for binary_name in mlg mlgc; do
  if [ -d "$bin_dir/$binary_name" ]; then
    echo "install destination is a directory: $bin_dir/$binary_name" >&2
    exit 1
  fi
done
staged_driver="$bin_dir/.mlg.install.$$"
staged_compiler="$bin_dir/.mlgc.install.$$"
cp "$extracted_driver" "$staged_driver"
cp "$extracted_compiler" "$staged_compiler"
chmod 0755 "$staged_driver" "$staged_compiler"

mv -f "$staged_compiler" "$bin_dir/mlgc"
staged_compiler=""
mv -f "$staged_driver" "$bin_dir/mlg"
staged_driver=""

printf 'installed Mallang %s to %s (mlg + mlgc)\n' "$version" "$bin_dir"
