# Spec: option-result-surface

## 목표

- `nil` 없이 optional value와 recoverable error를 표현할 v0 surface를 고정한다.
- parser/type checker/backend 구현 전에 `Option[T]`, `Result[T, E]`, constructor, `match`의 최소 규칙을 문서화한다.

## 범위

- `SPEC.md`에 타입 문법, constructor, pattern matching, ownership 규칙, 구현 staging을 정리한다.
- 이번 work unit은 docs-only다. parser/type checker 구현은 다음 work unit으로 분리한다.

## C-체크리스트

| ID | 상태 | Verify command | 작업 항목 |
| --- | --- | --- | --- |
| C1 | done | `scripts/check.sh` | `Option[T]` / `Result[T, E]` surface spec을 확정한다. |
| C2 | done | `scripts/check.sh` | 다음 구현 slice의 decision boundary를 handoff/roadmap에 남긴다. |

## 완료 기준

- C-체크리스트가 완료되고 검증 명령이 통과한다.
