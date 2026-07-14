#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

CLANG_BIN="${CLANG:-clang}"
refresh=1

usage() {
  cat >&2 <<'EOF'
usage: scripts/check-generated-c-sanitizers.sh [--assume-generated]

Runs the normal Mallang smoke gate, then recompiles every successful generated C
example from scripts/check.sh with AddressSanitizer and UndefinedBehaviorSanitizer.

Use --assume-generated to skip scripts/check.sh and reuse existing target/mallang
generated C artifacts.
EOF
}

if [[ $# -gt 1 ]]; then
  usage
  exit 2
fi

if [[ $# -eq 1 ]]; then
  case "$1" in
    --assume-generated)
      refresh=0
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
fi

if [[ "$refresh" -eq 1 ]]; then
  scripts/check.sh >/dev/null
fi

labels=()
while IFS= read -r label; do
  labels+=("$label")
done < <(
  sed -nE 's#.*build examples/[A-Za-z0-9_.-]+\.mlg -o target/mallang/([^ ]+).*#\1#p' scripts/check.sh |
    sort -u
)

if [[ "${#labels[@]}" -eq 0 ]]; then
  echo "no generated C example labels found in scripts/check.sh" >&2
  exit 1
fi

out_dir="target/mallang/generated-c-sanitizers"
mkdir -p "$out_dir"

count=0
for label in "${labels[@]}"; do
  c_source="target/mallang/${label}.c"
  native_binary="target/mallang/${label}"
  sanitizer_binary="${out_dir}/${label}"
  compile_stderr="${out_dir}/${label}.compile.stderr"
  native_stderr="${out_dir}/${label}.native.stderr"
  sanitizer_stderr="${out_dir}/${label}.sanitizer.stderr"

  if [[ ! -f "$c_source" || ! -x "$native_binary" ]]; then
    echo "missing generated artifacts for ${label}; rerun without --assume-generated" >&2
    exit 1
  fi

  if ! "$CLANG_BIN" \
    -fsanitize=address,undefined \
    -fno-omit-frame-pointer \
    "$c_source" \
    -o "$sanitizer_binary" \
    2>"$compile_stderr"; then
    echo "sanitizer compile failed for ${label}" >&2
    cat "$compile_stderr" >&2
    exit 1
  fi
  if [[ -s "$compile_stderr" ]]; then
    echo "sanitizer compile emitted stderr for ${label}" >&2
    cat "$compile_stderr" >&2
    exit 1
  fi

  if ! native_output="$("$native_binary" 2>"$native_stderr")"; then
    echo "native baseline run failed for ${label}" >&2
    cat "$native_stderr" >&2
    exit 1
  fi
  if [[ -s "$native_stderr" ]]; then
    echo "native baseline emitted stderr for ${label}" >&2
    cat "$native_stderr" >&2
    exit 1
  fi

  if ! sanitizer_output="$("$sanitizer_binary" 2>"$sanitizer_stderr")"; then
    echo "sanitizer run failed for ${label}" >&2
    cat "$sanitizer_stderr" >&2
    exit 1
  fi
  if [[ -s "$sanitizer_stderr" ]]; then
    echo "sanitizer emitted stderr for ${label}" >&2
    cat "$sanitizer_stderr" >&2
    exit 1
  fi
  if [[ "$sanitizer_output" != "$native_output" ]]; then
    echo "sanitizer output mismatch for ${label}" >&2
    echo "native output:" >&2
    printf '%s\n' "$native_output" >&2
    echo "sanitizer output:" >&2
    printf '%s\n' "$sanitizer_output" >&2
    exit 1
  fi

  count=$((count + 1))
done

echo "deep sanitizer generated C smoke passed for ${count} programs"
