# Spec: slice-element-borrow

## 목표

- Owned slice element를 기존 `con`/`mut` borrow argument surface에 연결한다.
- Copy-only `slice[i]` value access 제약은 유지하되, non-copy element도 call
  duration 동안 안전하게 빌릴 수 있게 한다.

## 범위

- `con values[i]`와 `mut values[i]`를 direct local slice source에 허용한다.
- Indexed slice element에서 이어지는 field path도 허용한다. 예:
  `con users[i].name`, `mut users[i].age`.
- Mutable slice element borrow는 root slice binding이 `mut`일 때만 허용한다.
- Slice index expression은 `int`여야 하고, 음수 literal은 `mlg check`에서
  reject한다.
- Native backend는 runtime `mlg_len` bounds guard 뒤에 element address를
  hidden-reference argument로 넘긴다.
- Same-call borrow overlap은 array element borrow와 같이 같은 root의 indexed
  place를 보수적으로 겹친 것으로 본다.

## 제외

- Slice element value extraction for non-Copy element types.
- Slice element assignment.
- Inline slice temporary borrow, 예: `con []int{1}[0]`.
- Borrowed slice views and first-class references.
- Mutable range values and by-reference range iteration.

## C-체크리스트

| ID | 상태 | Verify command | 작업 항목 |
| --- | --- | --- | --- |
| C1 | done | `cargo test --workspace slice_element_borrow` | semantic/IR/backend slice element borrow tests |
| C2 | done | `cargo run --bin mlg -- run examples/slice-element-borrow.mlg` | native slice element borrow smoke |
| C3 | done | `scripts/check.sh` | full repo smoke includes slice element borrow example |

## 완료 기준

- C-체크리스트가 완료되고 검증 명령이 통과한다.
