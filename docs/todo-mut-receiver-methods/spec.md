# Spec: mut-receiver-methods

## 목표

- 기존 receiver mode 설계 중 `mut` receiver method가 native backend에서 caller-visible mutation을 보장하는지 검증으로 고정한다.

## 범위

- `func (mut self T) method()` semantic 허용/거부 테스트를 추가한다.
- C backend가 `mut` receiver를 hidden mutable pointer로 선언하고 method call에서 receiver 주소를 넘기는지 unit test로 고정한다.
- `examples/mut-receiver.mlg` native smoke를 추가한다.
- 새 receiver syntax나 method values/dynamic dispatch는 도입하지 않는다.

## C-체크리스트

| ID | 상태 | Verify command | 작업 항목 |
| --- | --- | --- | --- |
| C1 | done | `scripts/check.sh` | mut receiver semantic/backend/native smoke 추가 |

## 완료 기준

- C-체크리스트가 완료되고 검증 명령이 통과한다.
