# Spec: field-index-cleanup-overwrite-drop

## 목표

- Cleanup type field/index assignment가 기존 owned value를 덮어쓰기 전에 old
  place를 drop하도록 IR cleanup insertion을 확장한다.
- Future slice 값이 struct field나 fixed array element에 들어갈 때 overwrite leak이
  생기지 않게 한다.

## 범위

- `IrStmtKind::FieldAssign`의 assigned value type이 cleanup type이면 assignment
  앞에 `IrStmtKind::Drop` field place를 삽입한다.
- `IrStmtKind::IndexAssign`의 assigned value type이 cleanup type이면 assignment
  앞에 `IrStmtKind::Drop` index place를 삽입한다.
- Backend가 explicit field/index drop place를 cleanup helper lvalue address로
  lower하는 regression을 고정한다.
- Source-level slice surface는 계속 reserved 상태로 둔다.

## 제외

- RHS-before-drop evaluation order를 보장하는 assignment statement redesign.
- Struct-wide drop helper for user structs.
- Partial move/destructuring.
- Source-level `drop(value)` syntax.
- Slice literal, `len(slice)`, `slice[i]`, `append` 구현.

## C-체크리스트

| ID | 상태 | Verify command | 작업 항목 |
| --- | --- | --- | --- |
| C1 | done | `cargo test --workspace cleanup_field` | field overwrite drop IR/backend regression |
| C2 | done | `cargo test --workspace cleanup_array_element` | array element overwrite drop IR/backend regression |
| C3 | done | `scripts/check.sh` | existing native surface and cleanup regressions 유지 |

## 완료 기준

- C-체크리스트가 완료되고 검증 명령이 통과한다.
