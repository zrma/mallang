# Spec: indexed-field-assignment

Status: complete; historical milestone record

## 목표

- Array/slice element에서 이어지는 field path를 assignment target으로 허용한다.
- Non-copy element를 값으로 추출하지 않고 checked lvalue place로 lowering한다.

## 범위

- `users[i].field = expr`와 `users[i].profile.name = expr`를 허용한다.
- Root는 direct local binding이어야 하며, mutable binding일 때만 field assignment가 가능하다.
- Fixed-size array index는 기존 compile-time literal/native bounds rule을 사용한다.
- Slice index는 direct local slice source에서만 허용하고, negative literal은
  `mlg check`에서 reject하며, upper bound는 native `mlg_len` guard로 확인한다.
- IR field assignment base는 assignment target expression으로 lowering한다.
- C backend는 indexed field assignment를 lvalue로 emit해 field slot에 직접 쓴다.

## 제외

- Borrowed indexing expressions and first-class references.
- Inline slice temporary assignment targets.
- Mutable range values and by-reference range iteration.
- Slice fields in structs.

## C-체크리스트

| ID | 상태 | Verify command | 작업 항목 |
| --- | --- | --- | --- |
| C1 | done | `cargo test --workspace indexed_field_assignment` | semantic/IR/backend indexed field assignment tests |
| C2 | done | `cargo run --bin mlg -- run examples/indexed-field-assignment.mlg` | native indexed field assignment smoke |
| C3 | done | `scripts/check.sh` | full repo smoke includes indexed field assignment example |

## 완료 기준

- C-체크리스트가 완료되고 검증 명령이 통과한다.
