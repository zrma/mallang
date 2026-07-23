#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

if [[ $# -ne 0 ]]; then
  echo "usage: scripts/check-self-hosting-default-compiler.sh" >&2
  exit 2
fi

if command -v cargo >/dev/null 2>&1; then
  cargo_command=(cargo)
elif command -v rustup >/dev/null 2>&1; then
  cargo_command=(rustup run stable cargo)
else
  echo "default compiler transition check failed: cargo is required" >&2
  exit 1
fi
command -v clang >/dev/null 2>&1 || {
  echo "default compiler transition check failed: clang is required" >&2
  exit 1
}

work="target/mallang/self-hosting/b5-default"
driver="target/debug/mlg"
self_compiler="target/debug/mlgc"
fixture="bootstrap/compiler/fixtures/backend/scalars.mlg"
semantic_rejection="bootstrap/compiler/fixtures/semantic/body-unknown-variable.mlg"
parser_rejection="bootstrap/compiler/fixtures/parser/recovery-statements.mlg"
mkdir -p "$work"

"${cargo_command[@]}" build --locked --quiet --lib --bin mlg
scripts/build-self-hosted-compiler.sh --stage0 "$driver" --output "$self_compiler" >/dev/null

crate_version="$(
  sed -n '/^\[package\]/,/^\[/ s/^version = "\([^"]*\)"/\1/p' Cargo.toml | head -n 1
)"
if [[ "$("$driver" --version)" != "mlg $crate_version" ]] || \
   [[ "$("$driver" --compiler stage0 --version)" != "mlg $crate_version" ]] || \
   [[ "$("$driver" --compiler self --version)" != "mlg $crate_version" ]] || \
   [[ "$("$driver" --compiler self --self-compiler "$self_compiler" --version)" != "mlg $crate_version" ]]; then
  echo "default compiler transition version compatibility failed" >&2
  exit 1
fi

stage0_provenance="$work/stage0.provenance"
self_provenance="$work/self.provenance"
"$driver" --compiler stage0 --version --verbose >"$stage0_provenance"
"$driver" --compiler self --version --verbose >"$self_provenance"
if [[ "$(cat "$stage0_provenance")" != $'mlg '"$crate_version"$'\ndriver: rust\ncompiler: stage0\ncore: rust-stage0' ]]; then
  echo "Stage0 compiler provenance mismatch" >&2
  cat "$stage0_provenance" >&2
  exit 1
fi
if [[ "$(cat "$self_provenance")" != $'mlg '"$crate_version"$'\ndriver: rust\ncompiler: self\ncore: mlgc protocol 1' ]]; then
  echo "self-hosted compiler provenance mismatch" >&2
  cat "$self_provenance" >&2
  exit 1
fi

stage0_binary="$work/scalars.stage0"
self_binary="$work/scalars.self"
"$driver" --compiler stage0 build "$fixture" -o "$stage0_binary" \
  >"$work/build.stage0.stdout" 2>"$work/build.stage0.stderr"
cp target/mallang/scalars.c "$work/scalars.stage0.c"
"$driver" --compiler self build "$fixture" -o "$self_binary" \
  >"$work/build.self.stdout" 2>"$work/build.self.stderr"
cp target/mallang/scalars.c "$work/scalars.self.c"
if [[ "$(cat "$work/build.stage0.stdout")" != "$stage0_binary" ]] || \
   [[ "$(cat "$work/build.self.stdout")" != "$self_binary" ]] || \
   [[ -s "$work/build.stage0.stderr" || -s "$work/build.self.stderr" ]] || \
   ! cmp -s "$work/scalars.stage0.c" "$work/scalars.self.c"; then
  echo "public Stage0/self build parity failed" >&2
  exit 1
fi

set +e
"$driver" --compiler stage0 run "$fixture" \
  >"$work/run.stage0.stdout" 2>"$work/run.stage0.stderr"
stage0_status=$?
"$driver" --compiler self run "$fixture" \
  >"$work/run.self.stdout" 2>"$work/run.self.stderr"
self_status=$?
set -e
if [[ "$stage0_status" -ne 0 || "$self_status" -ne 0 ]] || \
   [[ "$stage0_status" -ne "$self_status" ]] || \
   ! cmp -s "$work/run.stage0.stdout" "$work/run.self.stdout" || \
   ! cmp -s "$work/run.stage0.stderr" "$work/run.self.stderr"; then
  echo "public Stage0/self run parity failed" >&2
  exit 1
fi

"$driver" --compiler stage0 check "$fixture" \
  >"$work/check.stage0.stdout" 2>"$work/check.stage0.stderr"
"$driver" --compiler self check "$fixture" \
  >"$work/check.self.stdout" 2>"$work/check.self.stderr"
if ! cmp -s "$work/check.stage0.stdout" "$work/check.self.stdout" || \
   ! cmp -s "$work/check.stage0.stderr" "$work/check.self.stderr"; then
  echo "public Stage0/self check success parity failed" >&2
  exit 1
fi

for diagnostic_format in human json; do
  format_args=()
  if [[ "$diagnostic_format" == "json" ]]; then
    format_args=(--diagnostic-format json)
  fi
  for rejection in "$semantic_rejection" "$parser_rejection"; do
    name="$(basename "$rejection" .mlg).$diagnostic_format"
    set +e
    "$driver" "${format_args[@]}" --compiler stage0 check "$rejection" \
      >"$work/$name.stage0.stdout" 2>"$work/$name.stage0.stderr"
    stage0_status=$?
    "$driver" "${format_args[@]}" --compiler self check "$rejection" \
      >"$work/$name.self.stdout" 2>"$work/$name.self.stderr"
    self_status=$?
    set -e
    if [[ "$stage0_status" -eq 0 || "$self_status" -eq 0 ]] || \
       [[ "$stage0_status" -ne "$self_status" ]] || \
       ! cmp -s "$work/$name.stage0.stdout" "$work/$name.self.stdout" || \
       ! cmp -s "$work/$name.stage0.stderr" "$work/$name.self.stderr"; then
      echo "public Stage0/self $diagnostic_format rejection parity failed: $rejection" >&2
      exit 1
    fi
  done
done

if "$driver" --compiler self --self-compiler "$work/missing-mlgc" \
  build "$fixture" -o "$work/missing" \
  >"$work/missing.stdout" 2>"$work/missing.stderr"; then
  echo "missing self-hosted compiler unexpectedly fell back to Stage0" >&2
  exit 1
fi
if [[ -s "$work/missing.stdout" ]] || \
   ! grep -Fq 'self-hosted compiler not found' "$work/missing.stderr"; then
  echo "missing self-hosted compiler diagnostic mismatch" >&2
  exit 1
fi

echo "B5 default compiler transition gate passed: core=mlgc protocol=1 commands=check,build,run diagnostics=human,json fallback=explicit-only"
