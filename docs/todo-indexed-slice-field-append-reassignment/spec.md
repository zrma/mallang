# Spec: indexed-slice-field-append-reassignment

## 목표

- Indexed slice field를 같은 indexed field에 즉시 재저장하는 append 표면을 연다.
- General field partial move 없이
  `store.bags[i].values = append(store.bags[i].values, item)`를 안전하게 native
  lowering한다.

## 범위

- Semantic:
  - matched path의 index expression이 stable expression이면
    `store.bags[i].values = append(store.bags[i].values, item)`를 허용한다.
  - root binding은 기존 field assignment 규칙대로 `mut`이어야 한다.
  - mismatched source index와 call index는 P59 field-take append source로 허용한다.
  - general field partial move는 계속 reject한다.
  - slice indexed field assignment source 제약을 direct local slice에서
    local-rooted slice place로 완화한다.
- IR cleanup:
  - 같은 indexed field append reassignment에서는 overwritten field drop과 별도
    cleanup RHS temp를 생략한다.
- Backend:
  - 기존 `SliceAppend` realloc lowering과 indexed assignment lvalue lowering을
    재사용한다.
  - realloc된 source header를 old field로 별도 drop하지 않는다.
- Native smoke:
  - `examples/indexed-slice-field-append.mlg`가 indexed field path append를 검증한다.

## 제외

- Moving a slice field into another owner without same-statement reassignment is
  handled as source-field take append in P59.
- Call, `if`, `match`, or cleanup-allocating literals in matched same-field
  index expressions. Non-matched field source append uses P59 take semantics.
- First-class references and statement-spanning borrow lifetimes.

## C-체크리스트

| ID | 상태 | Verify command | 작업 항목 |
| --- | --- | --- | --- |
| C1 | done | `cargo test --workspace allows_append_to_reassign_same_indexed_slice_field` | semantic indexed same-field append reassignment 허용 |
| C2 | done | `cargo test --workspace ir_lowers_indexed_slice_field_append_without_overwrite_drop` | IR cleanup indexed overwrite drop 생략 |
| C3 | done | `cargo run --bin mlg -- run examples/indexed-slice-field-append.mlg` | native indexed slice field append smoke |
| C4 | done | `scripts/check.sh` | full repo smoke includes indexed slice field append example |
