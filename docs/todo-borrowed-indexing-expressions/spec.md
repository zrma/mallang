# Spec: borrowed-indexing-expressions

Status: complete; historical milestone record

## 목표

- Local-rooted fixed-size array and direct local slice indexing can be used in
  read-only borrow expression contexts even when the element type is non-copy.
  P55 expands the slice source side to local-rooted slice places.
- `print(users[i])`, `print(users[i].name)`, and `age := users[i].age` are valid
  without moving `users[i]` out of the array/slice.
- Owned extraction such as `user := users[i]` or `consume(users[i])` still
  requires a `Copy` element type.

## 범위

- Semantic: `ValueUse::Borrow` index access permits non-copy elements, while
  `ValueUse::Owned` keeps the existing Copy requirement.
- Semantic: moving a non-copy field out of an indexed element remains rejected.
- IR/backend: existing indexed lvalue/value lowering is reused for borrowed
  read contexts.
- Native smoke: indexed array/slice field reads are covered by
  `examples/indexed-field-read.mlg`.

## 제외

- First-class references or storing borrowed indexed values.
- Moving non-copy elements or non-copy fields out of indexed storage.
- Statement-spanning borrow lifetimes.
- Mutable range values.

## C-체크리스트

| ID | 상태 | Verify command | 작업 항목 |
| --- | --- | --- | --- |
| C1 | done | `cargo test --workspace borrowed` | semantic read/move boundary coverage |
| C2 | done | `cargo run --bin mlg -- run examples/indexed-field-read.mlg` | native indexed field read smoke |
| C3 | done | `scripts/check.sh` | full repo smoke includes indexed field read example |
