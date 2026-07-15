#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

usage() {
  cat <<'EOF'
usage: scripts/build-release-artifact.sh [--version <major.minor.patch>] [--output-dir <directory>]
EOF
}

fail_usage() {
  echo "$1" >&2
  usage >&2
  exit 2
}

version=""
output_dir="target/mallang/release-artifacts"
while [[ $# -gt 0 ]]; do
  case "$1" in
    --version)
      [[ $# -ge 2 && -n "$2" && "$2" != --* ]] || fail_usage "missing value for --version"
      version="$2"
      shift 2
      ;;
    --output-dir)
      [[ $# -ge 2 && -n "$2" && "$2" != --* ]] || fail_usage "missing value for --output-dir"
      output_dir="$2"
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

command -v python3 >/dev/null 2>&1 || {
  echo "python3 is required to build a deterministic release archive" >&2
  exit 1
}

cargo_bin="${CARGO_BIN:-cargo}"
command -v "$cargo_bin" >/dev/null 2>&1 || {
  echo "cargo executable not found: $cargo_bin" >&2
  exit 1
}

crate_version="$({
  sed -n '/^\[package\]/,/^\[/ s/^version = "\([^"]*\)"/\1/p' Cargo.toml
} | head -n 1)"
if [[ ! "$crate_version" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]]; then
  echo "Cargo package version must be major.minor.patch, got: $crate_version" >&2
  exit 1
fi
if [[ -z "$version" ]]; then
  version="$crate_version"
elif [[ ! "$version" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]]; then
  fail_usage "invalid release version: $version"
elif [[ "$version" != "$crate_version" ]]; then
  echo "release version $version does not match Cargo package version $crate_version" >&2
  exit 1
fi

case "$(uname -s):$(uname -m)" in
  Darwin:arm64|Darwin:aarch64)
    target="aarch64-apple-darwin"
    ;;
  Linux:x86_64|Linux:amd64)
    target="x86_64-unknown-linux-gnu"
    ;;
  *)
    echo "unsupported release host: $(uname -s) $(uname -m)" >&2
    exit 1
    ;;
esac

CARGO_TARGET_DIR="$ROOT/target" "$cargo_bin" build --release --locked --bin mlg
binary="$ROOT/target/release/mlg"
if [[ ! -x "$binary" ]]; then
  echo "release binary was not produced at $binary" >&2
  exit 1
fi
version_output="$("$binary" --version)"
if [[ "$version_output" != "mlg $version" ]]; then
  echo "release binary version mismatch: expected mlg $version, got $version_output" >&2
  exit 1
fi

mkdir -p target/mallang "$output_dir"
staging="$(mktemp -d "target/mallang/release-staging.XXXXXX")"
trap 'rm -rf "$staging"' EXIT
mkdir -p "$staging/bin"
cp "$binary" "$staging/bin/mlg"
chmod 0755 "$staging/bin/mlg"
cp LICENSE-MIT LICENSE-APACHE "$staging/"
cp packaging/README.md "$staging/README.md"
chmod 0644 "$staging/LICENSE-MIT" "$staging/LICENSE-APACHE" "$staging/README.md"

root_name="mallang-v${version}-${target}"
archive="$output_dir/${root_name}.tar.gz"
python3 scripts/create-release-archive.py \
  --source-dir "$staging" \
  --output "$archive" \
  --root-name "$root_name"

printf '%s\n' "$archive"
