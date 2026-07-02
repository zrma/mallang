# Spec: slice-element-assignment

## 목표

- Owned slice element를 direct indexed assignment target으로 허용한다.
- Fixed-size array element assignment와 같은 ownership/drop rule을 slice에
  확장하되, slice length가 runtime header 값이라는 차이를 native guard로
  처리한다.

## 범위

- `values[i] = expr`에서 `values`가 direct mutable local `[]T` 또는 `mut`
  slice parameter일 수 있다.
- `T`가 `Copy`이든 non-copy이든 RHS owned value를 slice slot으로 move한다.
- `i`는 `int`여야 하며, 음수 literal은 `mlg check`에서 reject한다.
- Native backend는 RHS 평가 전에 `i`를 temp에 저장하고 `mlg_len`으로 runtime
  bounds check를 수행한다.
- Element type이 cleanup resource이면 RHS를 temp로 먼저 평가한 뒤 old element를
  drop하고 slot을 overwrite한다.
- `for` body와 `for` post target에서 같은 indexed assignment rule을 사용한다.

## 제외

- Indexed field assignment, 예: `users[i].name = "kim"`.
- Non-copy element extraction as a value.
- Borrowed indexing expressions and first-class references.
- Mutable range values and by-reference range iteration.
- Inline slice temporary assignment target.

## C-체크리스트

| ID | 상태 | Verify command | 작업 항목 |
| --- | --- | --- | --- |
| C1 | done | `cargo test --workspace slice_element_assignment` | semantic/IR/backend slice element assignment tests |
| C2 | done | `cargo run --bin mlg -- run examples/slice-element-assignment.mlg` | native slice element assignment smoke |
| C3 | done | `scripts/check.sh` | full repo smoke includes slice element assignment example |

## 완료 기준

- C-체크리스트가 완료되고 검증 명령이 통과한다.
