# Spec: else-if-sugar

## 목표

- statement-form `if`와 expression-form `if`에서 `else if` sugar를 지원한다.
- AST/IR/backend에는 새 node를 추가하지 않고 nested `if`로 파싱한다.

## 범위

- 허용: `if cond { ... } else if other { ... } else { ... }` statement.
- 허용: `if cond { a } else if other { b } else { c }` expression.
- `else if`는 nested `if`로 파싱한다.
- 제외: block expression generalization.
- 제외: match statement/block arm sugar.

## C-체크리스트

| ID | 상태 | Verify command | 작업 항목 |
| --- | --- | --- | --- |
| C1 | done | `cargo test parser::tests::` | statement/expression `else if` parser test 추가 |
| C2 | done | `scripts/check.sh` | `examples/else-if.mlg` native smoke 추가 |
| C3 | done | `scripts/check.sh` | README/SPEC/ROADMAP/HANDOFF 상태 갱신 |

## 완료 기준

- C-체크리스트가 완료되고 검증 명령이 통과한다.
