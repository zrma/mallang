# Spec: match-expression

Status: complete; historical milestone record

## 목표

- Built-in ADT인 `Option[T]`와 `Result[T, E]`를 값으로 분해하는 exhaustive `match` expression을 추가한다.
- functional value style을 유지하기 위해 branch는 우선 expression을 반환한다.

## 범위

- Syntax: `match <expr> { case Some(name) { <expr> } case None { <expr> } }`.
- `Option[T]` match는 `Some(name)`과 `None` arm을 정확히 하나씩 요구한다.
- `Result[T, E]` match는 `Ok(name)`과 `Err(name)` arm을 정확히 하나씩 요구한다.
- payload binding은 해당 branch expression 안에서만 유효하다.
- 모든 branch expression은 같은 non-`unit` 타입이어야 한다.
- tagged typed IR와 C backend codegen은 다음 work unit으로 분리한다.

## C-체크리스트

| ID | 상태 | Verify command | 작업 항목 |
| --- | --- | --- | --- |
| C1 | done | `scripts/check.sh` | `match` expression AST/parser를 추가한다. |
| C2 | done | `scripts/check.sh` | `Option` / `Result` exhaustive type checking을 추가한다. |
| C3 | done | `scripts/check.sh` | IR/backend 미구현 경계를 명확한 diagnostic으로 남긴다. |
| C4 | done | `scripts/check.sh` | handoff/roadmap의 다음 boundary를 tagged IR/backend layout으로 이동한다. |

## 완료 기준

- C-체크리스트가 완료되고 검증 명령이 통과한다.
