# Spec: print-statement-only

## 목표

- `print(expr)`를 v0의 statement-only built-in으로 고정한다.
- `mlg check`가 `print`를 value position에서 사용하는 코드를 reject한다.
- `print` argument에는 `con` 또는 `mut` mode marker를 허용하지 않는다.

## 범위

- 허용: direct expression statement `print(expr)`.
- 거부: binding initializer, nested call argument, return expression, `if`
  expression branch, `match` expression arm 등 값이 필요한 위치의
  `print(...)`.
- 기존 printability rule은 유지한다.
- Backend-only invariant였던 statement-only `print` 에러를 semantic 단계로
  앞당긴다.

## C-체크리스트

| ID | 상태 | Verify command | 작업 항목 |
| --- | --- | --- | --- |
| C1 | done | `cargo test --workspace rejects_print` | semantic regression 추가 |
| C2 | done | `scripts/check.sh` | `mlg check` failure smoke 추가 |

## 완료 기준

- C-체크리스트가 완료되고 검증 명령이 통과한다.
