# Spec: slice-field-take-append-source

Status: complete; historical milestone record

## 목표

- Same-field reassignment 없이 owned slice field를 `append` source로 사용할 수 있게 한다.
- Full partial-move tracking 없이 source field를 empty slice로 되돌리는 take semantics로
  cleanup 안전성을 유지한다.

## 범위

- Semantic:
  - `grown := append(bag.values, item)`를 허용한다.
  - `moved := append(store.bags[i].values, item)`처럼 local-rooted indexed field
    source를 허용한다.
  - `target = append(other.values, item)`처럼 source와 target이 다른 field여도
    source field를 empty로 되돌리는 take/assign으로 허용한다.
  - Direct local `values` append는 기존 move semantics를 유지한다.
- Backend:
  - field source lvalue를 temp slice header로 copy한다.
  - item expression을 평가한 뒤 source field에 empty slice header를 write한다.
  - append result가 consumed buffer를 소유하고, owning struct cleanup은 empty source
    field를 drop한다.
- Native smoke:
  - `examples/slice-field-take-append.mlg`가 result ownership과 source empty reset을
    검증한다.

## 제외

- General field partial moves where the source field is left uninitialized.
- Moving non-slice fields out of structs.
- First-class references and statement-spanning borrow lifetimes.

## C-체크리스트

| ID | 상태 | Verify command | 작업 항목 |
| --- | --- | --- | --- |
| C1 | done | `cargo test --workspace allows_append_from_slice_field_by_taking_source_field` | semantic field take append 허용 |
| C2 | done | `cargo test --workspace ir_lowers_slice_field_append_take_source` | IR append source field 형태 유지 |
| C3 | done | `cargo test --workspace generates_c_for_slice_field_append_take_source` | C backend source field empty reset |
| C4 | done | `cargo run --bin mlg -- run examples/slice-field-take-append.mlg` | native field take append smoke |
| C5 | done | `scripts/check.sh` | full repo smoke includes field take append example |
