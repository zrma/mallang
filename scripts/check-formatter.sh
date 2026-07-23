#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

BIN="${1:-target/debug/mlg}"
if [[ $# -gt 0 ]]; then
  shift
fi
MLG_ARGS=("$@")
WORK="target/mallang/formatter-smoke"
DIRECT="$WORK/direct.mlg"
EXPECTED="$WORK/direct.expected.mlg"
PROJECT="$WORK/project"
ATOMIC_PROJECT="$WORK/atomic-project"

if [[ ! -x "$BIN" ]]; then
  echo "formatter smoke binary is not executable: $BIN" >&2
  exit 1
fi

run_mlg() {
  "$BIN" "${MLG_ARGS[@]}" "$@"
}

rm -rf "$WORK"
mkdir -p "$PROJECT/src/pkg" "$PROJECT/tests/pkg" "$ATOMIC_PROJECT/src/pkg"

cat >"$DIRECT" <<'MLG'
// formatter smoke


func main(){ // body
values:=[]int{1,2,3} // values
print(values[0])}
MLG

cat >"$EXPECTED" <<'MLG'
// formatter smoke

func main() { // body
    values := []int{1, 2, 3} // values
    print(values[0])
}
MLG

cp "$DIRECT" "$WORK/direct.before.mlg"
if run_mlg fmt --check "$DIRECT" >"$WORK/direct-check.stdout" 2>"$WORK/direct-check.stderr"; then
  echo "formatter direct check smoke failed: expected non-zero exit" >&2
  exit 1
fi
if [[ -s "$WORK/direct-check.stdout" ]] || \
  ! grep -Fxq "$DIRECT: not formatted" "$WORK/direct-check.stderr" || \
  ! cmp -s "$DIRECT" "$WORK/direct.before.mlg"; then
  echo "formatter direct check smoke failed: output or input mutation mismatch" >&2
  exit 1
fi

run_mlg fmt "$DIRECT" >"$WORK/direct-write.stdout" 2>"$WORK/direct-write.stderr"
if [[ "$(cat "$WORK/direct-write.stdout")" != "$DIRECT: formatted" ]] || \
  [[ -s "$WORK/direct-write.stderr" ]] || \
  ! cmp -s "$DIRECT" "$EXPECTED"; then
  echo "formatter direct write smoke failed" >&2
  diff -u "$EXPECTED" "$DIRECT" >&2 || true
  exit 1
fi

run_mlg fmt --check "$DIRECT" >"$WORK/direct-clean.stdout" 2>"$WORK/direct-clean.stderr"
cp "$DIRECT" "$WORK/direct.canonical.mlg"
run_mlg fmt "$DIRECT" >"$WORK/direct-idempotent.stdout" 2>"$WORK/direct-idempotent.stderr"
if [[ -s "$WORK/direct-clean.stdout" || -s "$WORK/direct-clean.stderr" || \
  -s "$WORK/direct-idempotent.stdout" || -s "$WORK/direct-idempotent.stderr" ]] || \
  ! cmp -s "$DIRECT" "$WORK/direct.canonical.mlg"; then
  echo "formatter idempotence smoke failed" >&2
  exit 1
fi

cat >"$PROJECT/mallang.toml" <<'TOML'
[project]
name = "formatter-smoke"
TOML
cat >"$PROJECT/src/main.mlg" <<'MLG'
package main
func main(){print(1)}
MLG
cat >"$PROJECT/src/pkg/helper.mlg" <<'MLG'
package pkg
pub func helper()int{return 1}
MLG
cat >"$PROJECT/tests/pkg/helper_test.mlg" <<'MLG'
package pkg
test HelperWorks(){assert(helper()==1)}
MLG

if run_mlg fmt --check "$PROJECT" >"$WORK/project-check.stdout" 2>"$WORK/project-check.stderr"; then
  echo "formatter project check smoke failed: expected non-zero exit" >&2
  exit 1
fi
cat >"$WORK/project-check.expected" <<'OUT'
src/main.mlg: not formatted
src/pkg/helper.mlg: not formatted
tests/pkg/helper_test.mlg: not formatted
OUT
if [[ -s "$WORK/project-check.stdout" ]] || \
  ! cmp -s "$WORK/project-check.stderr" "$WORK/project-check.expected"; then
  echo "formatter project check smoke failed: paths are not deterministic" >&2
  exit 1
fi

run_mlg fmt "$PROJECT" >"$WORK/project-write.stdout" 2>"$WORK/project-write.stderr"
cat >"$WORK/project-write.expected" <<'OUT'
src/main.mlg: formatted
src/pkg/helper.mlg: formatted
tests/pkg/helper_test.mlg: formatted
OUT
if [[ -s "$WORK/project-write.stderr" ]] || \
  ! cmp -s "$WORK/project-write.stdout" "$WORK/project-write.expected"; then
  echo "formatter project write smoke failed: paths are not deterministic" >&2
  exit 1
fi
run_mlg fmt --check "$PROJECT" >"$WORK/project-clean.stdout" 2>"$WORK/project-clean.stderr"
if [[ -s "$WORK/project-clean.stdout" || -s "$WORK/project-clean.stderr" ]]; then
  echo "formatter clean project check smoke failed" >&2
  exit 1
fi

cat >"$ATOMIC_PROJECT/mallang.toml" <<'TOML'
[project]
name = "formatter-atomic"
TOML
cat >"$ATOMIC_PROJECT/src/main.mlg" <<'MLG'
package main
func main(){print(1)}
MLG
cat >"$ATOMIC_PROJECT/src/pkg/broken.mlg" <<'MLG'
package pkg
func broken( {
MLG
cp "$ATOMIC_PROJECT/src/main.mlg" "$WORK/atomic-main.before.mlg"
if run_mlg fmt "$ATOMIC_PROJECT" >"$WORK/atomic.stdout" 2>"$WORK/atomic.stderr"; then
  echo "formatter atomic failure smoke failed: expected non-zero exit" >&2
  exit 1
fi
if [[ -s "$WORK/atomic.stdout" ]] || \
  ! grep -Fq 'src/pkg/broken.mlg:' "$WORK/atomic.stderr" || \
  ! cmp -s "$ATOMIC_PROJECT/src/main.mlg" "$WORK/atomic-main.before.mlg"; then
  echo "formatter atomic failure smoke failed: diagnostics or no-write contract mismatch" >&2
  exit 1
fi

echo "formatter smoke passed"
