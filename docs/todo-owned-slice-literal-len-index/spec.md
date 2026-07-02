# Spec: owned-slice-literal-len-index

## 목표

- Source-level `[]T`를 owned move-only slice value로 연다.
- `[]T{...}` literal, `len(slice)`, Copy-only `slice[i]` value access를 native
  backend까지 구현한다.

## 범위

- Semantic `Type::Slice(Box<Type>)`를 source type reference에서 허용한다.
- Slice literals는 owned heap buffer `{ data, len, cap }`를 생성한다.
- Empty slice literal은 `data = NULL`, `len = 0`, `cap = 0`으로 lowering한다.
- `len(slice)`는 direct local slice를 읽고 `int`를 반환한다.
- `slice[i]`는 direct local slice에서 Copy element만 value로 읽는다.
- Slice index는 native bounds check를 수행한다.
- Slice-containing struct field는 struct cleanup support가 생길 때까지 reject한다.

## 제외

- `append(values, item)` growth builtin.
- Slice range.
- `con slice[i]` / `mut slice[i]` element borrow.
- Inline borrowed slice temporaries such as `len([]int{1})`.
- Struct cleanup for slice fields.

## C-체크리스트

| ID | 상태 | Verify command | 작업 항목 |
| --- | --- | --- | --- |
| C1 | done | `cargo test --workspace slice` | parser/semantic/IR/backend slice regression |
| C2 | done | `cargo run --bin mlg -- run examples/slices.mlg` | source-level native slice smoke |
| C3 | done | `scripts/check.sh` | 전체 parser/semantic/IR/backend/native smoke |

## 완료 기준

- C-체크리스트가 완료되고 검증 명령이 통과한다.
