#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

usage() {
  echo "usage: scripts/check-self-hosting-backend.sh [--assume-bootstrap] [--fixtures-only]" >&2
}

ASSUME_BOOTSTRAP=false
FIXTURES_ONLY=false
while [[ $# -gt 0 ]]; do
  case "$1" in
    --assume-bootstrap)
      ASSUME_BOOTSTRAP=true
      ;;
    --fixtures-only)
      FIXTURES_ONLY=true
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

if command -v cargo >/dev/null 2>&1; then
  CARGO=(cargo)
elif command -v rustup >/dev/null 2>&1; then
  CARGO=(rustup run stable cargo)
else
  echo "self-hosting backend check failed: cargo is required" >&2
  exit 1
fi

CLANG_BIN="${CLANG:-clang}"
command -v "$CLANG_BIN" >/dev/null 2>&1 || {
  echo "self-hosting backend check failed: clang is required" >&2
  exit 1
}

WORK="target/mallang/self-hosting/b3-backend"
STAGE0="target/debug/mlg"
STAGE1="target/mallang/self-hosting/b1-lexer/bootstrap-frontend"
PROJECT="bootstrap/compiler"
FIXTURES=(scalars owned-control composite-values adt-match control-flow-loops owned-overwrite slice-append borrowed-callables function-values)
PROJECT_FIXTURES=(dynamic-owned-string string-intrinsics platform-intrinsics)
RUNTIME_REJECTION_FIXTURES=(composite-bounds integer-division-zero)
PROJECT_RUNTIME_REJECTION_FIXTURES=(platform-exit-range)
ALLOCATION_REJECTION_FIXTURES=(adt-allocation-failure slice-append-allocation-failure)
PROJECT_ALLOCATION_REJECTION_FIXTURES=(dynamic-string-allocation-failure string-join-allocation-failure platform-os-args-allocation-failure platform-fs-read-allocation-failure)
BOUNDARY_REJECTION_FIXTURES=(unsupported-closure)
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

mkdir -p "$WORK"
started=$SECONDS

if [[ "$ASSUME_BOOTSTRAP" == false ]]; then
  "${CARGO[@]}" build --locked --quiet --lib --bin mlg
  "$STAGE0" fmt --check "$PROJECT"
  "$STAGE0" check "$PROJECT" >/dev/null
  "$STAGE0" build "$PROJECT" -o "$WORK/bootstrap-compiler" >/dev/null
  "$CLANG_BIN" "${OPTIMIZED_FLAGS[@]}" \
    "$PROJECT/target/mallang/bootstrap_compiler.c" -o "$STAGE1"
elif [[ ! -x "$STAGE1" ]]; then
  echo "self-hosting backend check failed: assumed Stage1 compiler is missing" >&2
  exit 1
fi

for name in "${FIXTURES[@]}"; do
  fixture="$PROJECT/fixtures/backend/$name.mlg"
  oracle_c="target/mallang/$name.c"
  stage1_c="$WORK/$name.stage1.c"
  stage1_c_second="$WORK/$name.stage1.second.c"

  "$STAGE0" build "$fixture" -o "$WORK/$name.stage0" >/dev/null
  "$STAGE1" c "$fixture" >"$stage1_c"
  "$STAGE1" c "$fixture" >"$stage1_c_second"

  if ! cmp -s "$oracle_c" "$stage1_c"; then
    echo "self-hosting backend generated C differs from Stage0: $name" >&2
    diff -u "$oracle_c" "$stage1_c" >&2 || true
    exit 1
  fi
  if ! cmp -s "$stage1_c" "$stage1_c_second"; then
    echo "self-hosting backend generated C is not deterministic: $name" >&2
    exit 1
  fi

  "$CLANG_BIN" "${OPTIMIZED_FLAGS[@]}" "$stage1_c" -o "$WORK/$name.stage1"
  "$CLANG_BIN" "${SANITIZER_FLAGS[@]}" "$stage1_c" -o "$WORK/$name.stage1-san"

  generated_c_abs="$(cd "$(dirname "$stage1_c")" && pwd)/$(basename "$stage1_c")"
  cat >"$WORK/$name-accounting.c" <<EOF
#define main mallang_fixture_main
#include "$generated_c_abs"
#undef main

int main(void) {
    if (mallang_live_allocation_count() != 0) {
        fprintf(stderr, "self-hosting backend accounting did not start at zero\n");
        return 2;
    }
    if (mallang_fixture_main() != 0) {
        fprintf(stderr, "self-hosting backend fixture returned a non-zero status\n");
        return 3;
    }
    if (mallang_live_allocation_count() != 0) {
        fprintf(stderr, "self-hosting backend fixture leaked allocations\n");
        return 4;
    }
    return 0;
}
EOF

  "$CLANG_BIN" "${OPTIMIZED_FLAGS[@]}" \
    "$WORK/$name-accounting.c" -o "$WORK/$name-accounting"
  "$CLANG_BIN" "${SANITIZER_FLAGS[@]}" \
    "$WORK/$name-accounting.c" -o "$WORK/$name-accounting-san"

  "$WORK/$name.stage0" >"$WORK/$name.stage0.stdout"
  "$WORK/$name.stage1" >"$WORK/$name.stage1.stdout"
  "$WORK/$name.stage1-san" >"$WORK/$name.stage1-san.stdout" \
    2>"$WORK/$name.stage1-san.stderr"
  "$WORK/$name-accounting" >"$WORK/$name-accounting.stdout" \
    2>"$WORK/$name-accounting.stderr"
  "$WORK/$name-accounting-san" >"$WORK/$name-accounting-san.stdout" \
    2>"$WORK/$name-accounting-san.stderr"

  for output in \
    "$WORK/$name.stage1.stdout" \
    "$WORK/$name.stage1-san.stdout" \
    "$WORK/$name-accounting.stdout" \
    "$WORK/$name-accounting-san.stdout"; do
    if ! cmp -s "$WORK/$name.stage0.stdout" "$output"; then
      echo "self-hosting backend native output mismatch: $output" >&2
      exit 1
    fi
  done

  expected=""
  case "$name" in
    scalars)
      expected=$'30\ntrue\n-10\n10\n2\ntrue\ntrue'
      ;;
    owned-control)
      expected=$'ready\nmiddle\nready\nready\nequal\ndifferent\n말랑'
      ;;
    composite-values)
      expected=$'5\n3\n2\nbundle\n7\n1'
      ;;
    adt-match)
      expected=$'4\n0\n7\n0\n1\n9\npair\n8\n1\n1'
      ;;
    control-flow-loops)
      expected=$'done\n18\n27\n7'
      ;;
    owned-overwrite)
      expected=$'2\nlee\nchoi'
      ;;
    slice-append)
      expected=$'2\n4\n6\n0\n8\nkim'
      ;;
    borrowed-callables)
      expected=$'1\n2\nkim\nlee\n2\n4'
      ;;
    function-values)
      expected=$'20\n22\n42\nkim\n1\n2'
      ;;
    *)
      echo "self-hosting backend fixture has no expected output: $name" >&2
      exit 1
      ;;
  esac
  if [[ "$(cat "$WORK/$name.stage0.stdout")" != "$expected" ]]; then
    echo "self-hosting backend fixture output mismatch: $name" >&2
    exit 1
  fi
  if [[ -s "$WORK/$name.stage1-san.stderr" || \
        -s "$WORK/$name-accounting.stderr" || \
        -s "$WORK/$name-accounting-san.stderr" ]]; then
    echo "self-hosting backend runtime emitted unexpected stderr: $name" >&2
    cat \
      "$WORK/$name.stage1-san.stderr" \
      "$WORK/$name-accounting.stderr" \
      "$WORK/$name-accounting-san.stderr" >&2
    exit 1
  fi
done

for name in "${PROJECT_FIXTURES[@]}"; do
  fixture_root="$PROJECT/fixtures/backend/$name/src"
  fixture="$fixture_root/main.mlg"
  unit_name="${name//-/_}"
  oracle_c="$WORK/$name.stage0.c"
  stage1_c="$WORK/$name.stage1.c"
  stage1_c_second="$WORK/$name.stage1.second.c"

  "$STAGE0" build "$fixture" -o "$WORK/$name.stage0" >/dev/null
  cp target/mallang/main.c "$oracle_c"
  "$STAGE1" c-project 1 "$unit_name" "$fixture_root" 0 "$fixture" >"$stage1_c"
  "$STAGE1" c-project 1 "$unit_name" "$fixture_root" 0 "$fixture" >"$stage1_c_second"

  if ! cmp -s "$oracle_c" "$stage1_c"; then
    echo "self-hosting backend generated project C differs from Stage0: $name" >&2
    diff -u "$oracle_c" "$stage1_c" >&2 || true
    exit 1
  fi
  if ! cmp -s "$stage1_c" "$stage1_c_second"; then
    echo "self-hosting backend generated project C is not deterministic: $name" >&2
    exit 1
  fi

  "$CLANG_BIN" "${OPTIMIZED_FLAGS[@]}" "$stage1_c" -o "$WORK/$name.stage1"
  "$CLANG_BIN" "${SANITIZER_FLAGS[@]}" "$stage1_c" -o "$WORK/$name.stage1-san"

  generated_c_abs="$(cd "$(dirname "$stage1_c")" && pwd)/$(basename "$stage1_c")"
  if [[ "$name" == platform-intrinsics ]]; then
    cat >"$WORK/$name-accounting.c" <<EOF
#define main mallang_fixture_main
#include "$generated_c_abs"
#undef main

int main(void) {
    char mlg_arg0[] = "platform-intrinsics";
    char *mlg_argv[] = { mlg_arg0, NULL };
    if (mallang_live_allocation_count() != 0) {
        fprintf(stderr, "self-hosting backend accounting did not start at zero\n");
        return 2;
    }
    if (mallang_fixture_main(1, mlg_argv) != 0) {
        fprintf(stderr, "self-hosting backend fixture returned a non-zero status\n");
        return 3;
    }
    if (mallang_live_allocation_count() != 0) {
        fprintf(stderr, "self-hosting backend fixture leaked allocations\n");
        return 4;
    }
    return 0;
}
EOF
  else
    cat >"$WORK/$name-accounting.c" <<EOF
#define main mallang_fixture_main
#include "$generated_c_abs"
#undef main

int main(void) {
    if (mallang_live_allocation_count() != 0) {
        fprintf(stderr, "self-hosting backend accounting did not start at zero\n");
        return 2;
    }
    if (mallang_fixture_main() != 0) {
        fprintf(stderr, "self-hosting backend fixture returned a non-zero status\n");
        return 3;
    }
    if (mallang_live_allocation_count() != 0) {
        fprintf(stderr, "self-hosting backend fixture leaked allocations\n");
        return 4;
    }
    return 0;
}
EOF
  fi

  "$CLANG_BIN" "${OPTIMIZED_FLAGS[@]}" \
    "$WORK/$name-accounting.c" -o "$WORK/$name-accounting"
  "$CLANG_BIN" "${SANITIZER_FLAGS[@]}" \
    "$WORK/$name-accounting.c" -o "$WORK/$name-accounting-san"

  "$WORK/$name.stage0" >"$WORK/$name.stage0.stdout"
  "$WORK/$name.stage1" >"$WORK/$name.stage1.stdout"
  "$WORK/$name.stage1-san" >"$WORK/$name.stage1-san.stdout" \
    2>"$WORK/$name.stage1-san.stderr"
  "$WORK/$name-accounting" >"$WORK/$name-accounting.stdout" \
    2>"$WORK/$name-accounting.stderr"
  "$WORK/$name-accounting-san" >"$WORK/$name-accounting-san.stdout" \
    2>"$WORK/$name-accounting-san.stderr"

  for output in \
    "$WORK/$name.stage1.stdout" \
    "$WORK/$name.stage1-san.stdout" \
    "$WORK/$name-accounting.stdout" \
    "$WORK/$name-accounting-san.stdout"; do
    if ! cmp -s "$WORK/$name.stage0.stdout" "$output"; then
      echo "self-hosting backend project native output mismatch: $output" >&2
      exit 1
    fi
  done

  expected=""
  case "$name" in
    dynamic-owned-string)
      expected=$'42\ntrue'
      ;;
    string-intrinsics)
      expected=$'5\n234\nstring byte index out of bounds\n가\nstring slice boundary splits a UTF-8 scalar\n1\n-1\na|한|z\n-42\ninvalid integer text\ninteger value out of range'
      ;;
    platform-intrinsics)
      expected=$'1\nplatform-file\nmissing\nstderr-ok'
      ;;
    *)
      echo "self-hosting backend project fixture has no expected output: $name" >&2
      exit 1
      ;;
  esac
  if [[ "$(cat "$WORK/$name.stage0.stdout")" != "$expected" ]]; then
    echo "self-hosting backend project fixture output mismatch: $name" >&2
    exit 1
  fi
  if [[ -s "$WORK/$name.stage1-san.stderr" || \
        -s "$WORK/$name-accounting.stderr" || \
        -s "$WORK/$name-accounting-san.stderr" ]]; then
    echo "self-hosting backend project runtime emitted unexpected stderr: $name" >&2
    cat \
      "$WORK/$name.stage1-san.stderr" \
      "$WORK/$name-accounting.stderr" \
      "$WORK/$name-accounting-san.stderr" >&2
    exit 1
  fi
done

for name in "${RUNTIME_REJECTION_FIXTURES[@]}"; do
  fixture="$PROJECT/fixtures/backend/$name.mlg"
  oracle_c="target/mallang/$name.c"
  stage1_c="$WORK/$name.stage1.c"
  stage1_c_second="$WORK/$name.stage1.second.c"

  "$STAGE0" build "$fixture" -o "$WORK/$name.stage0" >/dev/null
  "$STAGE1" c "$fixture" >"$stage1_c"
  "$STAGE1" c "$fixture" >"$stage1_c_second"
  cmp -s "$oracle_c" "$stage1_c" || {
    echo "self-hosting backend rejection C differs from Stage0: $name" >&2
    diff -u "$oracle_c" "$stage1_c" >&2 || true
    exit 1
  }
  cmp -s "$stage1_c" "$stage1_c_second" || {
    echo "self-hosting backend rejection C is not deterministic: $name" >&2
    exit 1
  }

  "$CLANG_BIN" "${OPTIMIZED_FLAGS[@]}" "$stage1_c" -o "$WORK/$name.stage1"
  "$CLANG_BIN" "${SANITIZER_FLAGS[@]}" "$stage1_c" -o "$WORK/$name.stage1-san"

  for binary in stage0 stage1 stage1-san; do
    set +e
    "$WORK/$name.$binary" >"$WORK/$name.$binary.stdout" \
      2>"$WORK/$name.$binary.stderr"
    status=$?
    set -e
    if [[ $status -ne 1 ]]; then
      echo "self-hosting backend rejection returned $status instead of 1: $name.$binary" >&2
      exit 1
    fi
    if [[ -s "$WORK/$name.$binary.stdout" ]]; then
      echo "self-hosting backend rejection emitted unexpected stdout: $name.$binary" >&2
      exit 1
    fi
    expected=""
    case "$name" in
      composite-bounds)
        expected="mallang runtime error: array index out of bounds"
        ;;
      integer-division-zero)
        expected="mallang runtime error: division by zero"
        ;;
      *)
        echo "self-hosting backend runtime rejection has no expected diagnostic: $name" >&2
        exit 1
        ;;
    esac
    if [[ "$(cat "$WORK/$name.$binary.stderr")" != "$expected" ]]; then
      echo "self-hosting backend rejection stderr mismatch: $name.$binary" >&2
      exit 1
    fi
  done
done

for name in "${PROJECT_RUNTIME_REJECTION_FIXTURES[@]}"; do
  fixture_root="$PROJECT/fixtures/backend/$name/src"
  fixture="$fixture_root/main.mlg"
  unit_name="${name//-/_}"
  oracle_c="$WORK/$name.stage0.c"
  stage1_c="$WORK/$name.stage1.c"
  stage1_c_second="$WORK/$name.stage1.second.c"

  "$STAGE0" build "$fixture" -o "$WORK/$name.stage0" >/dev/null
  cp target/mallang/main.c "$oracle_c"
  "$STAGE1" c-project 1 "$unit_name" "$fixture_root" 0 "$fixture" >"$stage1_c"
  "$STAGE1" c-project 1 "$unit_name" "$fixture_root" 0 "$fixture" >"$stage1_c_second"
  if ! cmp -s "$oracle_c" "$stage1_c"; then
    echo "self-hosting backend project rejection C differs from Stage0: $name" >&2
    diff -u "$oracle_c" "$stage1_c" >&2 || true
    exit 1
  fi
  if ! cmp -s "$stage1_c" "$stage1_c_second"; then
    echo "self-hosting backend project rejection C is not deterministic: $name" >&2
    exit 1
  fi

  "$CLANG_BIN" "${OPTIMIZED_FLAGS[@]}" "$stage1_c" -o "$WORK/$name.stage1"
  "$CLANG_BIN" "${SANITIZER_FLAGS[@]}" "$stage1_c" -o "$WORK/$name.stage1-san"

  expected=""
  case "$name" in
    platform-exit-range)
      expected="mallang runtime error: process exit code out of range"
      ;;
    *)
      echo "self-hosting backend project runtime rejection has no expected diagnostic: $name" >&2
      exit 1
      ;;
  esac

  for binary in stage0 stage1 stage1-san; do
    set +e
    "$WORK/$name.$binary" >"$WORK/$name.$binary.stdout" \
      2>"$WORK/$name.$binary.stderr"
    status=$?
    set -e
    if [[ $status -ne 1 ]]; then
      echo "self-hosting backend project rejection returned $status instead of 1: $name.$binary" >&2
      exit 1
    fi
    if [[ -s "$WORK/$name.$binary.stdout" ]]; then
      echo "self-hosting backend project rejection emitted unexpected stdout: $name.$binary" >&2
      exit 1
    fi
    if [[ "$(cat "$WORK/$name.$binary.stderr")" != "$expected" ]]; then
      echo "self-hosting backend project rejection stderr mismatch: $name.$binary" >&2
      exit 1
    fi
  done
done

for name in "${ALLOCATION_REJECTION_FIXTURES[@]}"; do
  fixture="$PROJECT/fixtures/backend/$name.mlg"
  oracle_c="target/mallang/$name.c"
  stage1_c="$WORK/$name.stage1.c"
  stage1_c_second="$WORK/$name.stage1.second.c"

  "$STAGE0" build "$fixture" -o "$WORK/$name.stage0-default" >/dev/null
  "$STAGE1" c "$fixture" >"$stage1_c"
  "$STAGE1" c "$fixture" >"$stage1_c_second"
  cmp -s "$oracle_c" "$stage1_c" || {
    echo "self-hosting backend allocation rejection C differs from Stage0: $name" >&2
    diff -u "$oracle_c" "$stage1_c" >&2 || true
    exit 1
  }
  cmp -s "$stage1_c" "$stage1_c_second" || {
    echo "self-hosting backend allocation rejection C is not deterministic: $name" >&2
    exit 1
  }

  "$CLANG_BIN" "${OPTIMIZED_FLAGS[@]}" -DMLG_ALLOCATION_FAIL_AFTER=0 \
    "$oracle_c" -o "$WORK/$name.stage0"
  "$CLANG_BIN" "${OPTIMIZED_FLAGS[@]}" -DMLG_ALLOCATION_FAIL_AFTER=0 \
    "$stage1_c" -o "$WORK/$name.stage1"
  "$CLANG_BIN" "${SANITIZER_FLAGS[@]}" -DMLG_ALLOCATION_FAIL_AFTER=0 \
    "$stage1_c" -o "$WORK/$name.stage1-san"

  expected=""
  case "$name" in
    adt-allocation-failure)
      expected="mallang runtime error: recursive enum allocation failed"
      ;;
    slice-append-allocation-failure)
      expected="mallang runtime error: slice allocation failed"
      ;;
    *)
      echo "self-hosting backend allocation rejection has no expected diagnostic: $name" >&2
      exit 1
      ;;
  esac

  for binary in stage0 stage1 stage1-san; do
    set +e
    "$WORK/$name.$binary" >"$WORK/$name.$binary.stdout" \
      2>"$WORK/$name.$binary.stderr"
    status=$?
    set -e
    if [[ $status -ne 1 ]]; then
      echo "self-hosting backend allocation rejection returned $status instead of 1: $name.$binary" >&2
      exit 1
    fi
    if [[ -s "$WORK/$name.$binary.stdout" ]]; then
      echo "self-hosting backend allocation rejection emitted unexpected stdout: $name.$binary" >&2
      exit 1
    fi
    if [[ "$(cat "$WORK/$name.$binary.stderr")" != "$expected" ]]; then
      echo "self-hosting backend allocation rejection stderr mismatch: $name.$binary" >&2
      exit 1
    fi
  done
done

for name in "${PROJECT_ALLOCATION_REJECTION_FIXTURES[@]}"; do
  fixture_root="$PROJECT/fixtures/backend/$name/src"
  fixture="$fixture_root/main.mlg"
  unit_name="${name//-/_}"
  oracle_c="$WORK/$name.stage0.c"
  stage1_c="$WORK/$name.stage1.c"
  stage1_c_second="$WORK/$name.stage1.second.c"

  "$STAGE0" build "$fixture" -o "$WORK/$name.stage0-default" >/dev/null
  cp target/mallang/main.c "$oracle_c"
  "$STAGE1" c-project 1 "$unit_name" "$fixture_root" 0 "$fixture" >"$stage1_c"
  "$STAGE1" c-project 1 "$unit_name" "$fixture_root" 0 "$fixture" >"$stage1_c_second"
  if ! cmp -s "$oracle_c" "$stage1_c"; then
    echo "self-hosting backend project allocation rejection C differs from Stage0: $name" >&2
    diff -u "$oracle_c" "$stage1_c" >&2 || true
    exit 1
  fi
  if ! cmp -s "$stage1_c" "$stage1_c_second"; then
    echo "self-hosting backend project allocation rejection C is not deterministic: $name" >&2
    exit 1
  fi

  "$CLANG_BIN" "${OPTIMIZED_FLAGS[@]}" -DMLG_ALLOCATION_FAIL_AFTER=0 \
    "$oracle_c" -o "$WORK/$name.stage0"
  "$CLANG_BIN" "${OPTIMIZED_FLAGS[@]}" -DMLG_ALLOCATION_FAIL_AFTER=0 \
    "$stage1_c" -o "$WORK/$name.stage1"
  "$CLANG_BIN" "${SANITIZER_FLAGS[@]}" -DMLG_ALLOCATION_FAIL_AFTER=0 \
    "$stage1_c" -o "$WORK/$name.stage1-san"

  expected=""
  case "$name" in
    dynamic-string-allocation-failure)
      expected="mallang runtime error: string allocation failed"
      ;;
    string-join-allocation-failure)
      expected="mallang runtime error: joined string allocation failed"
      ;;
    platform-os-args-allocation-failure)
      expected="mallang runtime error: process argument allocation failed"
      ;;
    platform-fs-read-allocation-failure)
      expected="mallang runtime error: file path allocation failed"
      ;;
    *)
      echo "self-hosting backend project allocation rejection has no expected diagnostic: $name" >&2
      exit 1
      ;;
  esac
  for binary in stage0 stage1 stage1-san; do
    set +e
    "$WORK/$name.$binary" >"$WORK/$name.$binary.stdout" \
      2>"$WORK/$name.$binary.stderr"
    status=$?
    set -e
    if [[ $status -ne 1 ]]; then
      echo "self-hosting backend project allocation rejection returned $status instead of 1: $name.$binary" >&2
      exit 1
    fi
    if [[ -s "$WORK/$name.$binary.stdout" ]]; then
      echo "self-hosting backend project allocation rejection emitted unexpected stdout: $name.$binary" >&2
      exit 1
    fi
    if [[ "$(cat "$WORK/$name.$binary.stderr")" != "$expected" ]]; then
      echo "self-hosting backend project allocation rejection stderr mismatch: $name.$binary" >&2
      exit 1
    fi
  done
done

for name in "${BOUNDARY_REJECTION_FIXTURES[@]}"; do
  fixture="$PROJECT/fixtures/backend/$name.mlg"
  first="$WORK/$name.first.stdout"
  second="$WORK/$name.second.stdout"
  first_stderr="$WORK/$name.first.stderr"
  second_stderr="$WORK/$name.second.stderr"

  "$STAGE1" c "$fixture" >"$first" 2>"$first_stderr"
  "$STAGE1" c "$fixture" >"$second" 2>"$second_stderr"
  expected=""
  case "$name" in
    unsupported-closure)
      expected="B3 C backend does not yet support closures"
      ;;
    *)
      echo "self-hosting backend boundary fixture has no expected diagnostic: $name" >&2
      exit 1
      ;;
  esac
  if [[ "$(cat "$first")" != "$expected" ]]; then
    echo "self-hosting backend boundary rejection mismatch: $name" >&2
    exit 1
  fi
  if ! cmp -s "$first" "$second"; then
    echo "self-hosting backend boundary rejection is not deterministic: $name" >&2
    exit 1
  fi
  if [[ -s "$first_stderr" || -s "$second_stderr" ]]; then
    echo "self-hosting backend boundary rejection emitted unexpected stderr: $name" >&2
    exit 1
  fi
done

if [[ "$FIXTURES_ONLY" == false ]]; then
  compiler_sources=()
  while IFS= read -r source_path; do
    compiler_sources+=("$source_path")
  done < <(find "$PROJECT/src" -type f -name '*.mlg' -print | LC_ALL=C sort)

  compiler_c="$WORK/bootstrap-compiler.stage1.c"
  compiler_c_second="$WORK/bootstrap-compiler.stage1.second.c"
  "$STAGE1" c-project \
    1 bootstrap_compiler "$PROJECT/src" 0 "${compiler_sources[@]}" \
    >"$compiler_c"
  "$STAGE1" c-project \
    1 bootstrap_compiler "$PROJECT/src" 0 "${compiler_sources[@]}" \
    >"$compiler_c_second"
  if ! cmp -s "$compiler_c" "$compiler_c_second"; then
    echo "self-hosting backend compiler-project C is not deterministic" >&2
    exit 1
  fi
  "$CLANG_BIN" "${OPTIMIZED_FLAGS[@]}" \
    "$compiler_c" -o "$WORK/bootstrap-compiler.stage1"
fi

fixture_count=$((${#FIXTURES[@]} + ${#PROJECT_FIXTURES[@]}))
runtime_rejections=$((${#RUNTIME_REJECTION_FIXTURES[@]} + ${#PROJECT_RUNTIME_REJECTION_FIXTURES[@]} + ${#ALLOCATION_REJECTION_FIXTURES[@]} + ${#PROJECT_ALLOCATION_REJECTION_FIXTURES[@]}))
echo "self-hosting B3 backend gate passed: fixtures=$fixture_count runtime-rejections=$runtime_rejections boundary-rejections=${#BOUNDARY_REJECTION_FIXTURES[@]} compiler-project=$([[ "$FIXTURES_ONLY" == true ]] && echo skipped || echo strict) elapsed=$((SECONDS - started))s"
