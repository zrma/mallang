# Spec: explicit-drop-ir-backend

## 목표

- Automatic cleanup insertion 전에 typed IR/backend가 명시적 drop statement를
  표현하고 lower할 수 있게 한다.
- Source-level syntax는 추가하지 않고 internal IR capability로만 둔다.

## 범위

- `IrStmtKind::Drop { expr }`를 추가한다.
- C backend는 cleanup lvalue를 `mlg_drop_<Type>(&place);`로 lower한다.
- Drop target은 local/field/index place처럼 existing borrow lvalue lowering이
  처리할 수 있는 expression으로 제한한다.
- Drop target type이 cleanup type이 아니면 IR invariant error를 낸다.

## 제외

- Semantic/lowerer가 scope exit, early return, reassignment 지점에 drop을
  자동 삽입하는 lifetime lowering.
- Move analysis 기반 double-drop 방지.
- Source-level `drop(value)` syntax.

## C-체크리스트

| ID | 상태 | Verify command | 작업 항목 |
| --- | --- | --- | --- |
| C1 | done | `cargo test --workspace drop` | explicit internal drop statement backend regression |
| C2 | done | `scripts/check.sh` | existing native surface and reserved slice boundary 유지 |

## 완료 기준

- C-체크리스트가 완료되고 검증 명령이 통과한다.
