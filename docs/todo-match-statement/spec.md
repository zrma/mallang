# Spec: match-statement

## 목표

- statement-form `match`를 추가하고 각 arm이 block statements를 갖도록 한다.
- `Option` / `Result` exhaustive pattern rule과 branch-local payload binding을
  expression-form `match`와 동일하게 적용한다.
- statement-form `match`를 typed IR와 C backend까지 native smoke로 연결한다.

## 범위

- 허용: `match value { case Some(x) { print(x) } case None { ... } }`
  statement.
- 허용: arm block 안의 multiple statements.
- 허용: statement-form `match`가 모든 arm에서 return할 때 function
  return-completeness로 인정.
- 허용: non-local scrutinee expression은 C backend에서 temp로 한 번만 평가.
- 거부: non-exhaustive `Option` / `Result` statement match.
- 제외: expression-form `match`의 block arm generalization.
- 제외: nested patterns, pattern guards, user-defined enum declarations.

## C-체크리스트

| ID | 상태 | Verify command | 작업 항목 |
| --- | --- | --- | --- |
| C1 | done | `cargo test parser::tests::` | statement-form `match` parser test 추가 |
| C2 | done | `cargo test semantic::tests::` | exhaustive/block scope/return-completeness semantic test 추가 |
| C3 | done | `cargo test --workspace` | typed IR와 C backend statement match test 추가 |
| C4 | done | `scripts/check.sh` | `examples/match-statement.mlg` native smoke와 문서 갱신 |

## 완료 기준

- C-체크리스트가 완료되고 검증 명령이 통과한다.
