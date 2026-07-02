# Spec: field-assignment

## 목표

- Go-like direct struct field assignment을 parser, semantic checker, typed IR,
  C backend까지 end-to-end로 추가한다.

## 범위

- Syntax: `user.age = 31`
- Assignment target은 v0에서 direct local struct field만 허용한다.
- Base binding은 `mut`이어야 한다.
- RHS는 field type과 일치해야 한다.
- Field assignment는 전체 struct를 move하지 않는다.
- Nested field assignment, field-level borrow arguments such as
  `rename(mut user.name)`, partial moves, field assignment inside by-reference
  receiver methods are out of scope for this work unit.

## C-체크리스트

| ID | 상태 | Verify command | 작업 항목 |
| --- | --- | --- | --- |
| C1 | done | `cargo test parser::tests::parses_field_assignment` | AST/parser field assignment statement 추가 |
| C2 | done | `cargo test semantic::tests::allows_field_assignment_on_mutable_struct_binding` | semantic mutability, field lookup, RHS type checking 추가 |
| C3 | done | `cargo test ir::tests::ir_lowers_field_assignment` | typed IR field assignment lowering 추가 |
| C4 | done | `scripts/check.sh` | C backend field assignment codegen, native smoke, 문서 갱신 |

## 완료 기준

- C-체크리스트가 완료되고 검증 명령이 통과한다.
