#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

release_archive="$(scripts/check-release-artifacts.sh)"
if [[ ! -f "$release_archive" ]]; then
  echo "v0.7 acceptance release archive is missing: $release_archive" >&2
  exit 1
fi

release_work="target/mallang/release-artifact-smoke"
mlg="$release_work/home/.local/bin/mlg"
work="target/mallang/v07-acceptance"
toolkit="$work/toolkit"
workflow="$work/workflow"
binary="$work/bin/workflow"

if [[ ! -x "$mlg" ]]; then
  echo "v0.7 acceptance installed compiler is missing: $mlg" >&2
  exit 1
fi

rm -rf "$work"
mkdir -p "$toolkit/src" "$workflow/src" "$workflow/tests" "$work/bin"

cat >"$toolkit/mallang.toml" <<'TOML'
[project]
name = "toolkit"
TOML
cat >"$toolkit/src/library.mlg" <<'MLG'
package main
pub func Double(value int)int{return value*2}
MLG

cat >"$workflow/mallang.toml" <<'TOML'
[project]
name = "workflow"

[dependencies]
toolkit = { path = "../toolkit" }
TOML
cat >"$workflow/src/main.mlg" <<'MLG'
package main
import "toolkit"
func main(){print(toolkit.Double(21))}
MLG
cat >"$workflow/tests/main_test.mlg" <<'MLG'
package main
import "toolkit"
test DependencyWorks(){assert(toolkit.Double(21)==42)}
MLG

cp "$toolkit/src/library.mlg" "$work/toolkit.before.mlg"
cp "$workflow/src/main.mlg" "$work/workflow-main.before.mlg"
cp "$workflow/tests/main_test.mlg" "$work/workflow-test.before.mlg"

if "$mlg" fmt --check "$toolkit" >"$work/toolkit-check.stdout" 2>"$work/toolkit-check.stderr"; then
  echo "v0.7 acceptance expected the new toolkit project to need formatting" >&2
  exit 1
fi
if [[ -s "$work/toolkit-check.stdout" ]] || \
  [[ "$(cat "$work/toolkit-check.stderr")" != "src/library.mlg: not formatted" ]] || \
  ! cmp -s "$toolkit/src/library.mlg" "$work/toolkit.before.mlg"; then
  echo "v0.7 acceptance toolkit no-write format check mismatch" >&2
  exit 1
fi

if "$mlg" fmt --check "$workflow" >"$work/workflow-check.stdout" 2>"$work/workflow-check.stderr"; then
  echo "v0.7 acceptance expected the new workflow project to need formatting" >&2
  exit 1
fi
cat >"$work/workflow-check.expected" <<'OUT'
src/main.mlg: not formatted
tests/main_test.mlg: not formatted
OUT
if [[ -s "$work/workflow-check.stdout" ]] || \
  ! cmp -s "$work/workflow-check.stderr" "$work/workflow-check.expected" || \
  ! cmp -s "$workflow/src/main.mlg" "$work/workflow-main.before.mlg" || \
  ! cmp -s "$workflow/tests/main_test.mlg" "$work/workflow-test.before.mlg"; then
  echo "v0.7 acceptance workflow no-write format check mismatch" >&2
  exit 1
fi

"$mlg" fmt "$toolkit" >"$work/toolkit-format.stdout" 2>"$work/toolkit-format.stderr"
"$mlg" fmt "$workflow" >"$work/workflow-format.stdout" 2>"$work/workflow-format.stderr"
if [[ "$(cat "$work/toolkit-format.stdout")" != "src/library.mlg: formatted" ]] || \
  [[ -s "$work/toolkit-format.stderr" ]]; then
  echo "v0.7 acceptance toolkit format output mismatch" >&2
  exit 1
fi
cat >"$work/workflow-format.expected" <<'OUT'
src/main.mlg: formatted
tests/main_test.mlg: formatted
OUT
if [[ -s "$work/workflow-format.stderr" ]] || \
  ! cmp -s "$work/workflow-format.stdout" "$work/workflow-format.expected"; then
  echo "v0.7 acceptance workflow format output mismatch" >&2
  exit 1
fi

cp "$toolkit/src/library.mlg" "$work/toolkit.canonical.mlg"
cp "$workflow/src/main.mlg" "$work/workflow-main.canonical.mlg"
cp "$workflow/tests/main_test.mlg" "$work/workflow-test.canonical.mlg"
"$mlg" fmt --check "$toolkit" >"$work/toolkit-clean.stdout" 2>"$work/toolkit-clean.stderr"
"$mlg" fmt --check "$workflow" >"$work/workflow-clean.stdout" 2>"$work/workflow-clean.stderr"
"$mlg" fmt "$toolkit" >"$work/toolkit-idempotent.stdout" 2>"$work/toolkit-idempotent.stderr"
"$mlg" fmt "$workflow" >"$work/workflow-idempotent.stdout" 2>"$work/workflow-idempotent.stderr"
if [[ -s "$work/toolkit-clean.stdout" || -s "$work/toolkit-clean.stderr" || \
  -s "$work/workflow-clean.stdout" || -s "$work/workflow-clean.stderr" || \
  -s "$work/toolkit-idempotent.stdout" || -s "$work/toolkit-idempotent.stderr" || \
  -s "$work/workflow-idempotent.stdout" || -s "$work/workflow-idempotent.stderr" ]] || \
  ! cmp -s "$toolkit/src/library.mlg" "$work/toolkit.canonical.mlg" || \
  ! cmp -s "$workflow/src/main.mlg" "$work/workflow-main.canonical.mlg" || \
  ! cmp -s "$workflow/tests/main_test.mlg" "$work/workflow-test.canonical.mlg"; then
  echo "v0.7 acceptance formatter determinism or idempotence mismatch" >&2
  exit 1
fi

"$mlg" check "$toolkit" >"$work/toolkit-compile.stdout" 2>"$work/toolkit-compile.stderr"
"$mlg" --diagnostic-format=json check "$workflow" \
  >"$work/workflow-compile.stdout" 2>"$work/workflow-compile.stderr"
if [[ "$(cat "$work/toolkit-compile.stdout")" != "$toolkit: ok" ]] || \
  [[ "$(cat "$work/workflow-compile.stdout")" != "$workflow: ok" ]] || \
  [[ -s "$work/toolkit-compile.stderr" || -s "$work/workflow-compile.stderr" ]]; then
  echo "v0.7 acceptance project check output mismatch" >&2
  exit 1
fi

"$mlg" test "$workflow" >"$work/test.stdout" 2>"$work/test.stderr"
expected_test=$'test workflow::DependencyWorks ... ok\ntest result: ok. 1 passed; 0 failed'
if [[ "$(cat "$work/test.stdout")" != "$expected_test" ]] || [[ -s "$work/test.stderr" ]]; then
  echo "v0.7 acceptance project test output mismatch" >&2
  cat "$work/test.stdout" >&2
  cat "$work/test.stderr" >&2
  exit 1
fi

"$mlg" build "$workflow" -o "$binary" >"$work/build.stdout" 2>"$work/build.stderr"
if [[ "$(cat "$work/build.stdout")" != "$binary" ]] || \
  [[ -s "$work/build.stderr" ]] || [[ ! -x "$binary" ]]; then
  echo "v0.7 acceptance native build output mismatch" >&2
  exit 1
fi
if [[ "$("$binary" 2>"$work/binary.stderr")" != "42" ]] || \
  [[ -s "$work/binary.stderr" ]]; then
  echo "v0.7 acceptance built program output mismatch" >&2
  exit 1
fi
if [[ "$("$mlg" run "$workflow" 2>"$work/run.stderr")" != "42" ]] || \
  [[ -s "$work/run.stderr" ]]; then
  echo "v0.7 acceptance installed compiler run output mismatch" >&2
  exit 1
fi

echo "v0.7 clean-project format, check, test, build, install, and run acceptance passed"
