#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

usage() {
  echo "usage: scripts/check-v09-dogfood.sh [--compiler <path>]" >&2
  exit 2
}

compiler=""
while [[ $# -gt 0 ]]; do
  case "$1" in
    --compiler)
      [[ $# -ge 2 ]] || usage
      compiler="$2"
      shift 2
      ;;
    *)
      usage
      ;;
  esac
done

if [[ -z "$compiler" ]]; then
  release_archive="$(scripts/check-release-artifacts.sh)"
  if [[ ! -f "$release_archive" ]]; then
    echo "v0.9 dogfood release archive is missing: $release_archive" >&2
    exit 1
  fi
  compiler="target/mallang/release-artifact-smoke/home/.local/bin/mlg"
elif [[ "$compiler" != /* ]]; then
  compiler="$ROOT/$compiler"
fi

if [[ ! -x "$compiler" ]]; then
  echo "v0.9 dogfood compiler is missing or not executable: $compiler" >&2
  exit 1
fi

crate_version="$(sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml)"
if [[ "$($compiler --version)" != "mlg $crate_version" ]]; then
  echo "v0.9 dogfood compiler version mismatch" >&2
  exit 1
fi

work="target/mallang/v09-dogfood"
project="$work/textstats"
snapshot="$work/source-snapshot"
input="$ROOT/tests/fixtures/v06-reference-cli/input.txt"
expected="$work/expected.txt"
source_files="$work/source-files.txt"

rm -rf "$work"
mkdir -p "$project/src" "$project/tests" "$snapshot" "$work/bin"
cp examples/projects/textstats/mallang.toml "$project/mallang.toml"
cp -R examples/projects/textstats/src/. "$project/src/"
cp -R examples/projects/textstats/tests/. "$project/tests/"
printf 'bytes=12\nscalars=10\nlines=4\ndistinct_line_lengths=3\n' >"$expected"

find "$project" -type f \( -name '*.mlg' -o -name 'mallang.toml' \) \
  | sed "s#^$project/##" \
  | LC_ALL=C sort >"$source_files"
while IFS= read -r relative; do
  mkdir -p "$snapshot/$(dirname "$relative")"
  cp "$project/$relative" "$snapshot/$relative"
done <"$source_files"

check_sources_unchanged() {
  local actual_files="$work/source-files.actual.txt"
  local relative

  find "$project" -type f \( -name '*.mlg' -o -name 'mallang.toml' \) \
    | sed "s#^$project/##" \
    | LC_ALL=C sort >"$actual_files"
  if ! cmp -s "$source_files" "$actual_files"; then
    echo "v0.9 dogfood source file set changed" >&2
    exit 1
  fi
  while IFS= read -r relative; do
    if ! cmp -s "$snapshot/$relative" "$project/$relative"; then
      echo "v0.9 dogfood formatter changed canonical source: $relative" >&2
      exit 1
    fi
  done <"$source_files"
}

expected_test=$'test textstats/stats::SummarizesUnicodeText ... ok\ntest result: ok. 1 passed; 0 failed'
for iteration in 1 2; do
  prefix="$work/iteration-$iteration"
  binary="$work/bin/textstats-$iteration"

  "$compiler" fmt --check "$project" >"$prefix-fmt-check.stdout" 2>"$prefix-fmt-check.stderr"
  "$compiler" fmt "$project" >"$prefix-fmt.stdout" 2>"$prefix-fmt.stderr"
  if [[ -s "$prefix-fmt-check.stdout" || -s "$prefix-fmt-check.stderr" || \
    -s "$prefix-fmt.stdout" || -s "$prefix-fmt.stderr" ]]; then
    echo "v0.9 dogfood canonical formatter output mismatch on iteration $iteration" >&2
    exit 1
  fi
  check_sources_unchanged

  "$compiler" check "$project" >"$prefix-check.stdout" 2>"$prefix-check.stderr"
  if [[ "$(cat "$prefix-check.stdout")" != "$project: ok" ]] || \
    [[ -s "$prefix-check.stderr" ]]; then
    echo "v0.9 dogfood check output mismatch on iteration $iteration" >&2
    exit 1
  fi

  "$compiler" test "$project" >"$prefix-test.stdout" 2>"$prefix-test.stderr"
  if [[ "$(cat "$prefix-test.stdout")" != "$expected_test" ]] || \
    [[ -s "$prefix-test.stderr" ]]; then
    echo "v0.9 dogfood test output mismatch on iteration $iteration" >&2
    cat "$prefix-test.stdout" >&2
    cat "$prefix-test.stderr" >&2
    exit 1
  fi

  "$compiler" build "$project" -o "$binary" >"$prefix-build.stdout" 2>"$prefix-build.stderr"
  if [[ "$(cat "$prefix-build.stdout")" != "$binary" ]] || \
    [[ -s "$prefix-build.stderr" ]] || [[ ! -x "$binary" ]]; then
    echo "v0.9 dogfood build output mismatch on iteration $iteration" >&2
    exit 1
  fi

  "$binary" "$input" >"$prefix-binary.stdout" 2>"$prefix-binary.stderr"
  "$compiler" run "$project" -- "$input" >"$prefix-run.stdout" 2>"$prefix-run.stderr"
  if ! cmp -s "$expected" "$prefix-binary.stdout" || \
    ! cmp -s "$expected" "$prefix-run.stdout" || \
    [[ -s "$prefix-binary.stderr" || -s "$prefix-run.stderr" ]]; then
    echo "v0.9 dogfood runtime output mismatch on iteration $iteration" >&2
    exit 1
  fi

  output_file="$work/summary-$iteration.txt"
  "$binary" "$input" "$output_file" >"$prefix-file.stdout" 2>"$prefix-file.stderr"
  if ! cmp -s "$expected" "$output_file" || \
    [[ -s "$prefix-file.stdout" || -s "$prefix-file.stderr" ]]; then
    echo "v0.9 dogfood output-file mismatch on iteration $iteration" >&2
    exit 1
  fi

  set +e
  "$binary" >"$prefix-usage.stdout" 2>"$prefix-usage.stderr"
  usage_status=$?
  set -e
  if [[ "$usage_status" -ne 2 ]] || [[ -s "$prefix-usage.stdout" ]] || \
    [[ "$(cat "$prefix-usage.stderr")" != "usage: textstats <input> [output]" ]]; then
    echo "v0.9 dogfood usage failure mismatch on iteration $iteration" >&2
    exit 1
  fi

  generated_c="$project/target/mallang/textstats.c"
  if [[ "$iteration" -eq 1 ]]; then
    cp "$generated_c" "$work/textstats.c.canonical"
    scripts/check-reference-cli.sh "$generated_c" "$binary" "$input" >/dev/null
  elif ! cmp -s "$work/textstats.c.canonical" "$generated_c"; then
    echo "v0.9 dogfood generated C changed between iterations" >&2
    exit 1
  fi
done

if ! cmp -s "$work/iteration-1-binary.stdout" "$work/iteration-2-binary.stdout" || \
  ! cmp -s "$work/iteration-1-run.stdout" "$work/iteration-2-run.stdout" || \
  ! cmp -s "$work/iteration-1-test.stdout" "$work/iteration-2-test.stdout"; then
  echo "v0.9 dogfood observable output changed between iterations" >&2
  exit 1
fi

echo "v0.9 clean-install textstats format, check, test, build, and run dogfood passed twice"
