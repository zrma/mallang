# Spec: recursive-struct-type-check

Status: complete; historical milestone record

## 목표

- Recursive value type definitions를 `mlg check` 단계에서 reject한다.
- Native backend가 type emission 중에 발견하던 recursive type error를 semantic
  diagnostic으로 앞당긴다.

## 범위

- Semantic checker:
  - `type Node struct { next Node }` 같은 direct recursion을 reject한다.
  - `A -> B -> A` 같은 indirect recursion을 reject한다.
  - `Option[Node]`, `Result[Node, E]`, `[N]Node`처럼 wrapper 안에 들어간
    recursive struct reference도 v0 value type recursion으로 reject한다.
- Backend:
  - 기존 backend invariant guard는 유지한다.
- Smoke:
  - recursive struct source가 `mlg check`에서 non-zero로 실패하는지 검증한다.

## C-체크리스트

| ID | 상태 | Verify command | 작업 항목 |
| --- | --- | --- | --- |
| C1 | done | `scripts/check.sh` | direct recursive struct semantic reject |
| C2 | done | `scripts/check.sh` | indirect/wrapped recursive struct semantic reject |
| C3 | done | `scripts/check.sh` | docs/spec/roadmap/handoff 갱신 |

## 완료 기준

- C-체크리스트가 완료되고 검증 명령이 통과한다.
