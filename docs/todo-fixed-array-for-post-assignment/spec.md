# Spec: fixed-array-for-post-assignment

## 목표

- Go-like `for init; condition; post`의 post 절에서 fixed-size array element
  assignment를 허용한다.
- `values[i] = expr` statement와 동일한 mutable direct array, `Copy` element,
  bounds-check 규칙을 재사용한다.
- native C backend에서 단순 post assignment를 bounds-check된 C `for` header
  expression으로 생성한다.

## 범위

- Parser: `ForPost::Assign` target으로 `ExprKind::Index` 허용.
- Semantic: 기존 `check_index_assign` 규칙을 for post target에도 적용.
- IR: `IrForPost::Assign` target으로 `IrExprKind::Index` 허용.
- Backend: `mallang_check_index(index, len)` helper로 C header 안에서 runtime
  bounds check를 표현.
- Smoke: `examples/array-for-post.mlg`가 native build 후 `6`을 출력.
- 제외: target/RHS/condition lowering에 temporary prelude가 필요한 복잡한
  for-clause header expression 지원.

## C-체크리스트

| ID | 상태 | Verify command | 작업 항목 |
| --- | --- | --- | --- |
| C1 | done | `cargo test --workspace` | parser/semantic/IR/backend unit coverage |
| C2 | done | `scripts/check.sh` | native smoke including `examples/array-for-post.mlg` |

## 완료 기준

- C-체크리스트가 완료되고 검증 명령이 통과한다.
