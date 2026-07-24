# Spec: expression-branch-cleanup-normalization

Status: complete; historical milestone record

## 목표

- Expression-form `if`/`match`가 cleanup root를 branch별로 다르게 move할 때
  non-moving branch에 필요한 drop을 삽입한다.
- Statement cleanup insertion이 expression branch move root를 볼 수 있게 IR cleanup
  normalization을 expression 내부까지 확장한다.

## 범위

- `IrExprKind::If` branch에 `then_cleanup` / `else_cleanup` trailer를 추가한다.
- `IrMatchArm`에 arm-local `cleanup` trailer를 추가한다.
- Cleanup insertion에서 expression-form `if`/`match` branch moved roots를 merge한다.
- C backend는 expression cleanup trailer가 있으면 ternary 대신 temp block을 생성하고
  `temp = expr; cleanup;` 순서로 lowering한다.
- Source-level slice surface는 reserved 상태를 유지한다.

## C-체크리스트

| ID | 상태 | Verify command | 작업 항목 |
| --- | --- | --- | --- |
| C1 | done | `cargo test --workspace normalizes_cleanup_if_expression_branch_moves` | `if` expression branch move normalization regression |
| C2 | done | `cargo test --workspace normalizes_cleanup_match_expression_arm_moves` | `match` expression arm move normalization regression |
| C3 | done | `scripts/check.sh` | 전체 parser/semantic/IR/backend/native smoke |

## 완료 기준

- C-체크리스트가 완료되고 검증 명령이 통과한다.
