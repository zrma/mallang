# Spec: slice-field-append-reassignment

## 목표

- Owned slice field를 같은 field에 즉시 재저장하는 append 표면을 연다.
- General field partial move 없이 `bag.values = append(bag.values, item)`를
  안전하게 native lowering한다.

## 범위

- Semantic:
  - `bag.values = append(bag.values, item)` 허용.
  - `shelf.bag.values = append(shelf.bag.values, item)`처럼 indexed segment가
    없는 direct field path 허용.
  - root binding은 기존 field assignment 규칙대로 `mut`이어야 한다.
  - `grown := append(bag.values, item)` 같은 field partial move는 계속 reject.
- IR cleanup:
  - 같은 field append reassignment에서는 overwritten field drop과 별도 cleanup
    RHS temp를 생략한다.
  - direct local `values = append(values, item)`의 move-root cleanup 규칙은 유지한다.
- Backend:
  - 기존 `SliceAppend` realloc lowering을 재사용한다.
  - 같은 field append reassignment에서 realloc된 source header를 double-drop하지 않는다.
- Native smoke:
  - `examples/slice-field-append.mlg`가 direct/nested field path append를 검증한다.

## 제외

- Indexed field append sources, such as
  `store.bags[i].values = append(store.bags[i].values, item)`.
- Moving a slice field into another owner without same-statement reassignment.
- First-class references and statement-spanning borrow lifetimes.

## C-체크리스트

| ID | 상태 | Verify command | 작업 항목 |
| --- | --- | --- | --- |
| C1 | done | `cargo test --workspace allows_append_to_reassign_same_slice_field` | semantic same-field append reassignment 허용 |
| C2 | done | `cargo test --workspace ir_lowers_slice_field_append_without_overwrite_drop` | IR cleanup field overwrite drop 생략 |
| C3 | done | `cargo run --bin mlg -- run examples/slice-field-append.mlg` | native slice field append smoke |
| C4 | done | `scripts/check.sh` | full repo smoke includes slice field append example |
