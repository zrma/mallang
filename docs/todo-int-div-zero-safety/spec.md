# Spec: int-div-zero-safety

## 목표

- Integer division and remainder가 C backend에서 undefined behavior에 기대지
  않도록 zero divisor safety를 추가한다.
- `mlg check`에서 명확한 literal zero divisor는 조기에 reject한다.
- dynamic divisor는 native runtime guard가 Mallang runtime error로 종료한다.

## 범위

- Semantic checker:
  - `/`와 `%`의 right operand가 integer literal `0`이면 reject한다.
  - non-literal divisor는 semantic 단계에서 허용하고 backend guard에 맡긴다.
- Backend:
  - `/`와 `%` right operand를 temp로 한 번만 평가한다.
  - temp가 `0`이면 `mallang runtime error: division by zero`를 stderr에 쓰고
    `exit(1)`로 종료한다.
  - 기존 prelude ordering과 expression side effect single-evaluation을 유지한다.
- Smoke:
  - non-zero `/`와 `%` 결과를 native smoke로 검증한다.
  - dynamic zero divisor가 `mlg run`에서 non-zero로 실패하는지 검증한다.
- 이번 slice에서는 integer overflow, `INT64_MIN / -1`, checked arithmetic
  전반은 다루지 않는다.

## C-체크리스트

| ID | 상태 | Verify command | 작업 항목 |
| --- | --- | --- | --- |
| C1 | done | `scripts/check.sh` | semantic literal zero divisor reject |
| C2 | done | `scripts/check.sh` | backend runtime guard for dynamic `/` and `%` |
| C3 | done | `scripts/check.sh` | native smoke and docs/spec/roadmap/handoff 갱신 |

## 완료 기준

- C-체크리스트가 완료되고 검증 명령이 통과한다.
