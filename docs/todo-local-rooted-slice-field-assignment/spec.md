# Spec: local-rooted-slice-field-assignment

## 목표

- Struct field로 소유한 slice element를 직접 갱신할 수 있게 한다.
- 기존 direct mutable slice assignment를 local-rooted mutable indexed place로 확장한다.
- Cleanup element overwrite 순서는 기존 slice element assignment와 동일하게 유지한다.

## 범위

- Semantic:
  - `bag.values[i] = expr`처럼 local-rooted mutable array/slice source의 indexed assignment를 허용한다.
  - Root binding이 immutable이면 reject한다.
  - Slice index는 `int`여야 하고 negative literal은 `mlg check`에서 reject한다.
- IR:
  - Index assignment base를 assignment target expression으로 lowering한다.
  - Cleanup element overwrite는 RHS temp, old element drop, slot assignment 순서를 유지한다.
- Backend:
  - Index assignment base를 generic assignment lvalue로 lowering한다.
  - Slice field element assignment에 runtime `mlg_len` bounds guard를 유지한다.
- Native smoke:
  - `examples/slice-field-assignment.mlg`에서 Copy element assignment와 cleanup element overwrite를 검증한다.

## 제외

- `append(bag.values, item)`처럼 field slice를 consuming append source로 쓰는
  경로는 P57에서 direct field path same-field reassignment로 제한해 완료됐다.
- Inline slice temporary assignment targets.
- First-class references and statement-spanning borrow lifetimes.

## C-체크리스트

| ID | 상태 | Verify command | 작업 항목 |
| --- | --- | --- | --- |
| C1 | done | `cargo test --workspace allows_local_rooted_slice_element_assignment` | semantic local-rooted indexed assignment 허용 |
| C2 | done | `cargo test --workspace generates_c_for_local_rooted_slice_element_assignment` | backend lvalue/bounds/drop lowering |
| C3 | done | `cargo run --bin mlg -- run examples/slice-field-assignment.mlg` | native slice field assignment smoke |
| C4 | done | `scripts/check.sh` | full repo smoke includes slice field assignment example |
