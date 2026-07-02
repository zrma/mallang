# Spec: borrowed-value-escape

## 목표

- `in`/`mut` parameter로 들어온 non-copy borrowed value가 owned position으로 escape하지 못하게 한다.
- SPEC의 “borrowed values cannot be stored/returned” 규칙을 현재 parameter modes 기준으로 검증 가능한 checker rule로 고정한다.

## 범위

- Function parameter와 receiver local에 borrowed origin을 기록한다.
- Non-copy borrowed local을 `let`, owned argument, `return`, non-copy `match` scrutinee 같은 owned use position에서 사용하면 reject한다.
- Copy type borrowed locals는 value copy를 허용한다.
- Reference 타입이나 statement-spanning borrow lifetime은 도입하지 않는다.

## C-체크리스트

| ID | 상태 | Verify command | 작업 항목 |
| --- | --- | --- | --- |
| C1 | done | `scripts/check.sh` | semantic checker에 borrowed non-copy move reject 추가 |
| C2 | done | `scripts/check.sh` | return/storage/owned argument reject와 copy allow tests 추가 |

## 완료 기준

- C-체크리스트가 완료되고 검증 명령이 통과한다.
