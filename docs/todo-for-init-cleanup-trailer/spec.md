# Spec: for-init-cleanup-trailer

## 목표

- `for init; condition; post` loop의 init binding이 cleanup type일 때, loop
  C scope 안에서 정리할 수 있는 cleanup trailer를 IR/backend에 추가한다.
- Source-level slice surface는 계속 reserved 상태로 둔다.

## 범위

- `IrStmtKind::For`에 loop-exit cleanup trailer를 추가한다.
- Cleanup type `for` init binding을 loop cleanup root로 추적한다.
- Normal loop exit와 `break` 이후에는 loop trailer에서 for-init cleanup root를
  drop한다.
- `continue`는 for-init cleanup root를 보존한다.
- loop body 안의 `return` 전에는 for-init cleanup root를 drop한다.
- C backend는 for-init binding과 같은 C block 안에서 cleanup trailer를 출력한다.

## 제외

- Loop body에서 for-init cleanup root가 move되는 runtime state tracking.
- Outer cleanup root가 loop body에서 move되는 runtime state tracking.
- `range` source cleanup ownership changes.
- Expression-form `if`/`match` branch cleanup normalization.
- Field/index assignment old-value drop insertion.
- Source-level `drop(value)` syntax.
- Slice literal, `len(slice)`, `slice[i]`, `append` 구현.

## C-체크리스트

| ID | 상태 | Verify command | 작업 항목 |
| --- | --- | --- | --- |
| C1 | done | `cargo test --workspace cleanup` | for-init cleanup root IR regression |
| C2 | done | `cargo test --workspace for_init_cleanup` | for-init cleanup backend trailer regression |
| C3 | done | `scripts/check.sh` | existing native surface and reserved slice boundary 유지 |

## 완료 기준

- C-체크리스트가 완료되고 검증 명령이 통과한다.
