# Spec: straight-line-cleanup-reassignment-drop

## 목표

- Straight-line cleanup insertion에서 cleanup root reassignment가 기존 owned
  resource를 덮어써서 누수되지 않도록 old value drop을 삽입한다.
- Source-level slice surface는 계속 reserved 상태로 둔다.

## 범위

- Active cleanup root에 cleanup value를 재대입할 때 assignment 직전에 기존
  root를 `IrStmtKind::Drop`으로 삽입한다.
- RHS owned position으로 move된 cleanup root는 active roots에서 제거한다.
- Reassigned target root는 새 value의 cleanup 대상으로 active 상태를 유지한다.

## 제외

- `if`, `match`, `for`, `break`, `continue` 내부 control-flow cleanup insertion.
- Field/index assignment old-value drop insertion.
- Source-level `drop(value)` syntax.
- Slice literal, `len(slice)`, `slice[i]`, `append` 구현.

## C-체크리스트

| ID | 상태 | Verify command | 작업 항목 |
| --- | --- | --- | --- |
| C1 | done | `cargo test --workspace cleanup` | straight-line cleanup reassignment regression |
| C2 | done | `scripts/check.sh` | existing native surface and reserved slice boundary 유지 |

## 완료 기준

- C-체크리스트가 완료되고 검증 명령이 통과한다.
