# Spec: slice-range

## 목표

- Owned slice를 Go-like `range` source로 허용한다.
- Array range와 같은 표면을 유지하되, slice backing allocation 안전성을
  해치지 않는 작은 범위로 연다.

## 범위

- `for i, value := range values`에서 `values`가 `[]T`일 수 있다.
- `value` binding은 `T`가 `Copy`일 때만 허용한다.
- `for i := range values`, `for i, _ := range values`, `for _ := range values`
  같은 index-only forms는 non-Copy element slice에도 허용한다.
- Slice range source는 direct local이어야 한다.
- Inline cleanup temporary source, 예: `range []int{1, 2}`, 는 reject한다.
- Range body에서 active range source binding에 assignment하는 것을 reject한다.
- Native backend는 copied slice header의 `mlg_len`으로 loop bound를 만든다.

## 제외

- `con slice[i]` / `mut slice[i]` element borrow.
- Mutable range values.
- By-reference range iteration.
- Borrowed slice views.
- Inline slice temporary cleanup.
- Slice fields in structs.

## C-체크리스트

| ID | 상태 | Verify command | 작업 항목 |
| --- | --- | --- | --- |
| C1 | done | `cargo test --workspace range` | semantic/IR/backend slice range tests |
| C2 | done | `cargo run --bin mlg -- run examples/slice-range.mlg` | native slice range smoke |
| C3 | done | `scripts/check.sh` | full repo smoke includes slice range example |

## 완료 기준

- C-체크리스트가 완료되고 검증 명령이 통과한다.
