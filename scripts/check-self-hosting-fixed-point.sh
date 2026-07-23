#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

usage() {
  cat >&2 <<'EOF'
usage: scripts/check-self-hosting-fixed-point.sh [--assume-bootstrap] [--skip-sanitizers] [--jobs <count>]

The argument-free command is the complete B4 deep gate. Skip options are for
diagnostic edit loops and are not milestone or publication evidence.
EOF
}

ASSUME_BOOTSTRAP=false
SKIP_SANITIZERS=false
JOBS="${SELF_HOSTING_JOBS:-}"
while [[ $# -gt 0 ]]; do
  case "$1" in
    --assume-bootstrap)
      ASSUME_BOOTSTRAP=true
      shift
      ;;
    --skip-sanitizers)
      SKIP_SANITIZERS=true
      shift
      ;;
    --jobs)
      if [[ $# -lt 2 ]]; then
        usage
        exit 2
      fi
      JOBS="$2"
      shift 2
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
done

if [[ -z "$JOBS" ]]; then
  JOBS="$(getconf _NPROCESSORS_ONLN 2>/dev/null || echo 1)"
  if ((JOBS > 4)); then
    JOBS=4
  fi
fi
if [[ ! "$JOBS" =~ ^[1-9][0-9]*$ ]]; then
  echo "self-hosting fixed-point jobs must be a positive integer: $JOBS" >&2
  exit 2
fi

if command -v cargo >/dev/null 2>&1; then
  CARGO=(cargo)
elif command -v rustup >/dev/null 2>&1; then
  CARGO=(rustup run stable cargo)
else
  echo "self-hosting fixed-point check failed: cargo is required" >&2
  exit 1
fi

CLANG_BIN="${CLANG:-clang}"
command -v "$CLANG_BIN" >/dev/null 2>&1 || {
  echo "self-hosting fixed-point check failed: clang is required" >&2
  exit 1
}

WORK="target/mallang/self-hosting/b4-fixed-point"
PROJECT="bootstrap/compiler"
STAGE0="target/debug/mlg"
STAGE1="target/mallang/self-hosting/b1-lexer/bootstrap-frontend"
STAGE0_C="$PROJECT/target/mallang/bootstrap_compiler.c"
STAGE2_C="$WORK/bootstrap-compiler.stage2.c"
STAGE2="$WORK/bootstrap-compiler.stage2"
FIXED_C="$WORK/bootstrap-compiler.fixed.c"
OPTIMIZED_FLAGS=(-std=c11 -O2 -Wall -Wextra -Werror -pedantic)
SANITIZER_FLAGS=(
  -std=c11
  -O1
  -Wall
  -Wextra
  -Werror
  -pedantic
  "-fsanitize=address,undefined"
  -fno-omit-frame-pointer
)

mkdir -p "$WORK" "$(dirname "$STAGE1")"
started=$SECONDS

if [[ "$ASSUME_BOOTSTRAP" == false ]]; then
  "${CARGO[@]}" build --locked --quiet --lib --bin mlg
  "$STAGE0" fmt --check "$PROJECT"
  "$STAGE0" check "$PROJECT" >/dev/null
  "$STAGE0" build "$PROJECT" -o "$WORK/bootstrap-compiler.stage1" >/dev/null
  "$CLANG_BIN" "${OPTIMIZED_FLAGS[@]}" "$STAGE0_C" -o "$STAGE1"
elif [[ ! -x "$STAGE1" ]]; then
  echo "self-hosting fixed-point check failed: assumed Stage1 compiler is missing" >&2
  exit 1
fi

compiler_sources=()
while IFS= read -r source_path; do
  compiler_sources+=("$source_path")
done < <(find "$PROJECT/src" -type f -name '*.mlg' -print | LC_ALL=C sort)

"$STAGE1" c-project \
  1 bootstrap_compiler "$PROJECT/src" 0 "${compiler_sources[@]}" \
  >"$STAGE2_C" 2>"$WORK/stage1-generation.stderr"
if [[ -s "$WORK/stage1-generation.stderr" ]]; then
  echo "self-hosting Stage1 compiler generation emitted stderr" >&2
  cat "$WORK/stage1-generation.stderr" >&2
  exit 1
fi
"$CLANG_BIN" "${OPTIMIZED_FLAGS[@]}" "$STAGE2_C" -o "$STAGE2"

"$STAGE2" c-project \
  1 bootstrap_compiler "$PROJECT/src" 0 "${compiler_sources[@]}" \
  >"$FIXED_C" 2>"$WORK/stage2-generation.stderr"
if [[ -s "$WORK/stage2-generation.stderr" ]]; then
  echo "self-hosting Stage2 compiler generation emitted stderr" >&2
  cat "$WORK/stage2-generation.stderr" >&2
  exit 1
fi
if ! cmp -s "$STAGE2_C" "$FIXED_C"; then
  echo "self-hosting compiler C did not reach a Stage1-to-Stage2 fixed point" >&2
  exit 1
fi

if [[ "$SKIP_SANITIZERS" == false ]]; then
  "$CLANG_BIN" "${SANITIZER_FLAGS[@]}" \
    "$STAGE2_C" -o "$WORK/bootstrap-compiler.stage2-san"
  ASAN_OPTIONS=abort_on_error=1:detect_leaks=0 \
    UBSAN_OPTIONS=halt_on_error=1 \
    "$WORK/bootstrap-compiler.stage2-san" \
    c-project 1 bootstrap_compiler "$PROJECT/src" 0 \
    "${compiler_sources[@]}" \
    >"$WORK/bootstrap-compiler.sanitized.c" \
    2>"$WORK/sanitized-generation.stderr"
  if [[ -s "$WORK/sanitized-generation.stderr" ]]; then
    echo "self-hosting sanitized compiler generation emitted stderr" >&2
    cat "$WORK/sanitized-generation.stderr" >&2
    exit 1
  fi
  if ! cmp -s "$STAGE2_C" "$WORK/bootstrap-compiler.sanitized.c"; then
    echo "self-hosting sanitized compiler output differs from the fixed point" >&2
    exit 1
  fi
fi

PAIR_TASKS="$WORK/compiler-pair-tasks.bin"
: >"$PAIR_TASKS"
pair_count=0
queue_pair_fixture() {
  local kind="$1"
  local fixture="$2"
  local stem="$3"
  printf '%s\0' \
    "$STAGE1" "$STAGE2" "$kind" "$fixture" "$WORK" "$stem" \
    >>"$PAIR_TASKS"
  pair_count=$((pair_count + 1))
}

for fixture in "$PROJECT/fixtures/lexer"/*.mlg; do
  queue_pair_fixture lexer "$fixture" \
    "lexer-$(basename "$fixture" .mlg)"
done
for fixture in "$PROJECT/fixtures/parser"/*.mlg; do
  queue_pair_fixture parser "$fixture" \
    "parser-$(basename "$fixture" .mlg)"
done
for fixture in "$PROJECT/fixtures/semantic"/*.mlg; do
  queue_pair_fixture semantic "$fixture" \
    "semantic-$(basename "$fixture" .mlg)"
done
for fixture in "$PROJECT/fixtures/ir"/*.mlg; do
  queue_pair_fixture ir "$fixture" \
    "ir-$(basename "$fixture" .mlg)"
done
for fixture in "$PROJECT/fixtures/ir-test"/*.mlg; do
  queue_pair_fixture ir-test "$fixture" \
    "ir-test-$(basename "$fixture" .mlg)"
done
for fixture in "$PROJECT/fixtures/backend"/*.mlg; do
  queue_pair_fixture c "$fixture" \
    "backend-$(basename "$fixture" .mlg)"
done

corpus_count=0
while IFS= read -r fixture; do
  queue_pair_fixture parser "$fixture" \
    "parser-corpus-$(printf '%04d' "$corpus_count")"
  corpus_count=$((corpus_count + 1))
done < <(
  find \
    "$PROJECT/src" \
    "$PROJECT/tests" \
    examples \
    tests/fixtures \
    -type f -name '*.mlg' -print | LC_ALL=C sort
)
if ((corpus_count < 150)); then
  echo "self-hosting fixed-point parser corpus unexpectedly small: $corpus_count" >&2
  exit 1
fi

xargs -0 -n 6 -P "$JOBS" \
  scripts/check-self-hosting-compiler-pair-fixture.sh <"$PAIR_TASKS"

compare_invocation() {
  local stem="$1"
  shift
  local -a invocation=("$@")
  local stage1_status
  local stage2_status

  set +e
  "$STAGE1" "${invocation[@]}" >"$WORK/$stem.stage1.stdout" \
    2>"$WORK/$stem.stage1.stderr"
  stage1_status=$?
  "$STAGE2" "${invocation[@]}" >"$WORK/$stem.stage2.stdout" \
    2>"$WORK/$stem.stage2.stderr"
  stage2_status=$?
  set -e

  printf '%s\n' "$stage1_status" >"$WORK/$stem.stage1.status"
  printf '%s\n' "$stage2_status" >"$WORK/$stem.stage2.status"
  for suffix in stdout stderr status; do
    if ! cmp -s "$WORK/$stem.stage1.$suffix" "$WORK/$stem.stage2.$suffix"; then
      echo "self-hosting compiler-pair invocation mismatch: $stem ($suffix)" >&2
      diff -u "$WORK/$stem.stage1.$suffix" "$WORK/$stem.stage2.$suffix" >&2 || true
      exit 1
    fi
  done
}

for operation in link prepare check ir; do
  compare_invocation \
    "compiler-project-$operation" \
    "$operation-project" \
    1 bootstrap_compiler "$PROJECT/src" 0 \
    "${compiler_sources[@]}"
done

backend_project_count=0
while IFS= read -r source_root; do
  fixture_root="$(dirname "$source_root")"
  name="$(basename "$fixture_root")"
  fixture="$source_root/main.mlg"
  unit_name="${name//-/_}"
  compare_invocation \
    "backend-project-$name" \
    c-project 1 "$unit_name" "$source_root" 0 "$fixture"
  backend_project_count=$((backend_project_count + 1))
done < <(
  find "$PROJECT/fixtures/backend" \
    -mindepth 2 -maxdepth 2 -type d -name src -print | LC_ALL=C sort
)

compiler_bytes="$(wc -c <"$STAGE2_C" | tr -d ' ')"
sanitizer_status="passed"
if [[ "$SKIP_SANITIZERS" == true ]]; then
  sanitizer_status="skipped"
fi
echo "self-hosting B4 fixed-point gate passed: bytes=$compiler_bytes fixtures=$pair_count parser-corpus=$corpus_count backend-projects=$backend_project_count sanitizer=$sanitizer_status elapsed=$((SECONDS - started))s"
