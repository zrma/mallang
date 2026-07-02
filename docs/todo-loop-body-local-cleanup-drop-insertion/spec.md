# Spec: loop-body-local-cleanup-drop-insertion

## 목표

- Cleanup drop insertion을 `for`/`range` loop body-local cleanup roots까지
  확장한다.
- `break`/`continue` 같은 loop exit path에서 body-local cleanup roots가
  누락되지 않게 한다.
- Source-level slice surface는 계속 reserved 상태로 둔다.

## 범위

- `for` body-local cleanup roots는 loop body tail에서 `IrStmtKind::Drop`으로
  정리한다.
- `range` body-local cleanup roots도 같은 loop body scope 규칙을 적용한다.
- `break`/`continue` 전에는 loop body-local cleanup roots를 drop한다.
- loop body 안의 `return` 전에는 outer cleanup roots와 loop body-local cleanup
  roots를 drop한다.
- `break`/`continue`는 pre-loop outer cleanup roots를 drop하지 않는다.

## 제외

- Outer cleanup root가 loop body에서 move되는 runtime state tracking.
- `for` init binding cleanup과 post expression cleanup.
- Expression-form `if`/`match` branch cleanup normalization.
- Field/index assignment old-value drop insertion.
- Source-level `drop(value)` syntax.
- Slice literal, `len(slice)`, `slice[i]`, `append` 구현.

## C-체크리스트

| ID | 상태 | Verify command | 작업 항목 |
| --- | --- | --- | --- |
| C1 | done | `cargo test --workspace cleanup` | for/range body-local cleanup and break/continue regression |
| C2 | done | `scripts/check.sh` | existing native surface and reserved slice boundary 유지 |

## 완료 기준

- C-체크리스트가 완료되고 검증 명령이 통과한다.
