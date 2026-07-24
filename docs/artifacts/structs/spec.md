# Spec: structs

Status: complete; historical milestone record

## 목표

- Go-like `type Name struct { ... }`, struct literal, field access를
  parser/semantic/typed IR/C backend까지 end-to-end로 추가한다.

## 범위

- Declaration syntax: `type User struct { name string age int }`
- Literal syntax: `User{name: "kim", age: 30}`
- Field access syntax: `user.name`
- Struct values are move-only in v0.
- Field access can be used for reading/printing fields. Moving a non-copy field
  out of a named struct value is rejected until destructuring or partial-move
  semantics is designed.
- Methods, field assignment, nested mutable borrow of fields, recursive
  by-value structs, and struct pattern matching are out of scope.

## C-체크리스트

| ID | 상태 | Verify command | 작업 항목 |
| --- | --- | --- | --- |
| C1 | done | `cargo test parser::tests::parses_struct_declaration_literal_and_field_access` | AST/parser struct declaration, literal, field access 추가 |
| C2 | done | `cargo test semantic::tests::allows_struct_literal_and_field_access` | semantic struct signature, literal field validation, field access 추가 |
| C3 | done | `cargo test ir::tests::ir_lowers_struct_literal_and_field_access` | typed IR struct definitions/literal/access 추가 |
| C4 | done | `scripts/check.sh` | C backend typedef/literal/access/native smoke와 문서 갱신 |

## 완료 기준

- C-체크리스트가 완료되고 검증 명령이 통과한다.
