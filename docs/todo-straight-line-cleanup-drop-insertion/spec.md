# Spec: straight-line-cleanup-drop-insertion

## 목표

- Automatic cleanup insertion의 첫 단계로 straight-line function body에서
  owned cleanup roots를 추적해 explicit `IrStmtKind::Drop`을 삽입한다.
- Source-level slice surface는 계속 reserved 상태로 둔다.

## 범위

- Owned cleanup parameters를 active cleanup roots로 등록한다.
- `let`으로 만들어진 cleanup locals를 active cleanup roots로 등록한다.
- Cleanup root가 owned position으로 move되면 active roots에서 제거한다.
- Function tail과 top-level `return` 전에 active cleanup roots를 역순으로
  `IrStmtKind::Drop`으로 삽입한다.
- Return expression으로 move되는 cleanup root는 drop 대상에서 제외한다.

## 제외

- `if`, `match`, `for`, `break`, `continue` 내부 control-flow cleanup insertion.
- Cleanup reassignment old-value drop insertion.
- Source-level `drop(value)` syntax.
- Slice literal, `len(slice)`, `slice[i]`, `append` 구현.

## C-체크리스트

| ID | 상태 | Verify command | 작업 항목 |
| --- | --- | --- | --- |
| C1 | done | `cargo test --workspace cleanup` | straight-line cleanup insertion regression |
| C2 | done | `scripts/check.sh` | existing native surface and reserved slice boundary 유지 |

## 완료 기준

- C-체크리스트가 완료되고 검증 명령이 통과한다.
