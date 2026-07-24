# Spec: slice-append-builtin

Status: complete; historical milestone record

## 목표

- Owned slice growth surface를 `append(slice, item)` built-in으로 연다.
- Go-like call syntax를 유지하되, 첫 인자 slice와 item은 모두 owned value로
  consume한다.
- Caller-visible update는 `mut values` local에
  `values = append(values, item)`로 표현한다.

## 범위

- Semantic: `append`는 정확히 두 개의 owned argument를 요구한다.
- Semantic: 첫 인자는 `[]T`, 둘째 인자는 `T`여야 한다.
- Semantic: `con`/`mut` mode marker는 `append` argument에서 허용하지 않는다.
- Ownership: append RHS가 target root를 move한 뒤 같은 target에 assign하면,
  assignment 후 binding을 initialized/cleanup-active 상태로 복구한다.
- Ownership: cleanup value를 expression statement로 버리는 형태는 temporary
  cleanup lowering 전까지 reject한다.
- IR: `SliceAppend` expression으로 user function call과 분리한다.
- Backend: C `realloc` growth path와 length/capacity update를 생성한다.
- Smoke: `examples/slice-append.mlg`가 native로 `9`를 출력한다.

## 제외

- Slice range.
- `con slice[i]` / `mut slice[i]` element borrow.
- Borrowed slice views.
- Slice fields in structs.
- By-reference or mutable range iteration.

## C-체크리스트

| ID | 상태 | Verify command | 작업 항목 |
| --- | --- | --- | --- |
| C1 | done | `cargo test --workspace append` | semantic/IR/backend append tests |
| C2 | done | `cargo run --bin mlg -- run examples/slice-append.mlg` | native append smoke |
| C3 | done | `scripts/check.sh` | full repo smoke includes append example |

## 완료 기준

- C-체크리스트가 완료되고 검증 명령이 통과한다.
