# Spec: overlapping-borrows

Status: complete; historical milestone record

## 목표

- same-call borrow conflict를 semantic checker에서 검출해 ownership-lite의 독점
  mutable borrow 규칙을 닫는다.

## 범위

- 같은 함수 호출의 argument list 안에서 동일 local에 대한 `mut` borrow가 다른
  `con`/`mut` borrow와 겹치면 reject한다.
- 같은 함수 호출에서 동일 local을 여러 번 `con` borrow하는 것은 허용한다.
- borrow argument는 기존 ownership-lite 규칙처럼 direct local variable만 허용한다.
- call duration 밖으로 borrow가 저장되는 모델은 아직 없으므로 statement 간 borrow
  lifetime tracking은 범위 밖이다.

## C-체크리스트

| ID | 상태 | Verify command | 작업 항목 |
| --- | --- | --- | --- |
| C1 | done | `scripts/check.sh` | same-call borrow conflict 구현 |
| C2 | done | `cargo test --workspace borrow_conflict` | overlap diagnostics 회귀 테스트 |

## 완료 기준

- C-체크리스트가 완료되고 검증 명령이 통과한다.

## 검증 증거

- `scripts/check.sh`
- `cargo test --workspace borrow_conflict`
