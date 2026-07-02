# Spec: checked-int-arithmetic

## 목표

- Native C backend의 signed integer arithmetic이 C undefined behavior에 기대지
  않도록 checked arithmetic을 추가한다.
- `mlg check`에서 명확한 literal overflow는 조기에 reject한다.
- dynamic overflow는 native runtime guard가 Mallang runtime error로 종료한다.

## 범위

- Semantic checker:
  - literal `+`, `-`, `*` overflow를 reject한다.
  - literal unary `-` overflow를 reject한다.
  - literal `INT64_MIN / -1` overflow를 reject한다.
  - constant folding 전체나 boolean comparison folding은 다루지 않는다.
- Backend:
  - `+`, `-`, `*`는 C compiler overflow builtin으로 checked lowering한다.
  - unary `-`는 checked negation으로 lowering한다.
  - `/`와 `%`는 기존 zero divisor guard에 더해 `INT64_MIN / -1` guard를 추가한다.
  - checked operator operands는 temp로 한 번만 평가한다.
- Smoke:
  - non-overflow arithmetic 결과를 native smoke로 검증한다.
  - dynamic add/subtract/multiply/negate/division overflow가 `mlg run`에서
    non-zero로 실패하는지 검증한다.

## C-체크리스트

| ID | 상태 | Verify command | 작업 항목 |
| --- | --- | --- | --- |
| C1 | done | `scripts/check.sh` | semantic literal arithmetic overflow reject |
| C2 | done | `scripts/check.sh` | backend runtime guard for dynamic checked arithmetic |
| C3 | done | `scripts/check.sh` | native smoke and docs/spec/roadmap/handoff 갱신 |

## 완료 기준

- C-체크리스트가 완료되고 검증 명령이 통과한다.
