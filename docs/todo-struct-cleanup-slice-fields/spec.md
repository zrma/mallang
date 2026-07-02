# Spec: struct-cleanup-slice-fields

## 목표

- Struct fields may own cleanup resources such as `[]T`.
- Struct values remain move-only and become cleanup-capable roots.
- Dropping a struct recursively drops fields whose types need cleanup.

## 범위

- Semantic: remove the v0 rejection for slice fields in structs.
- Type classification: `Type::Struct` participates in cleanup tracking.
- IR cleanup insertion: reuse existing local, reassignment, field overwrite,
  branch, loop, return, and owned parameter cleanup paths.
- C backend: emit `mlg_drop_Struct_*` helpers that call field drop helpers only
  for cleanup fields.
- Native smoke: `examples/struct-slice-field.mlg` covers local struct cleanup,
  cleanup field overwrite, root reassignment, and owned parameter cleanup.

## 제외

- Borrowed slice views.
- First-class references.
- Statement-spanning borrow lifetimes.
- `len(bag.values)` or direct built-in operations on slice fields; current slice
  built-ins still require direct local slice sources.

## C-체크리스트

| ID | 상태 | Verify command | 작업 항목 |
| --- | --- | --- | --- |
| C1 | done | `cargo test --workspace allows_slice_fields_with_struct_cleanup` | semantic slice field 허용 |
| C2 | done | `cargo test --workspace generates_c_drop_helpers_for_internal_cleanup_types` | struct drop helper emission |
| C3 | done | `cargo run --bin mlg -- run examples/struct-slice-field.mlg` | native struct slice field smoke |
| C4 | done | `scripts/check.sh` | full repo smoke includes struct slice field example |
