# Spec: loop-persistent-move-safety

Status: complete; historical milestone record

## 목표

- `for`/`range` loop에서 반복 iteration을 가로질러 살아있는 move-only
  binding을 loop 안에서 consume하는 경로를 v0 semantic checker에서 막는다.
- Cleanup runtime state tracking 없이 no-use-after-move와 no-double-drop
  invariant를 유지한다.

## 범위

- `for` condition, body, post가 loop-persistent move-only binding을 move하면
  semantic error로 reject한다.
- Three-clause `for` init binding은 loop-persistent binding으로 취급한다.
- `range` body가 outer move-only binding을 move하면 semantic error로 reject한다.
- Loop body 안에서 새로 만든 move-only local을 같은 iteration 안에서 move하는
  것은 계속 허용한다.
- Copy type binding은 기존처럼 loop 안에서 owned argument로 사용할 수 있다.

## 제외

- Runtime moved-state flag tracking.
- Partial move/destructuring.
- First-class references or borrowed iteration values.
- Source-level `drop(value)` syntax.
- Slice literal, `len(slice)`, `slice[i]`, `append` 구현.

## C-체크리스트

| ID | 상태 | Verify command | 작업 항목 |
| --- | --- | --- | --- |
| C1 | done | `cargo test --workspace persistent_move` | for/range persistent move reject regression |
| C2 | done | `scripts/check.sh` | existing native surface and cleanup regressions 유지 |

## 완료 기준

- C-체크리스트가 완료되고 검증 명령이 통과한다.
