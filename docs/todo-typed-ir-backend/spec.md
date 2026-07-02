# Spec: typed-ir-backend

## 목표

- semantic checker를 통과한 AST를 typed IR로 lower하고, C backend가 AST를 직접
  해석하지 않게 분리한다.

## 범위

- `CheckedProgram -> IrProgram` lowering 추가.
- IR node에 expression type을 명시적으로 보존한다.
- C backend는 `IrProgram`만 emit한다.
- public API는 기존 `generate_c(&Program)`을 유지하되 내부에서 `check -> lower -> emit`
  순서로 실행한다.
- 첫 native subset의 동작과 smoke output `30`은 유지한다.

## C-체크리스트

| ID | 상태 | Verify command | 작업 항목 |
| --- | --- | --- | --- |
| C1 | done | `scripts/check.sh` | typed IR lowering과 C backend 전환 |
| C2 | done | `cargo test --workspace ir` | IR lowering 회귀 테스트 |

## 완료 기준

- C-체크리스트가 완료되고 검증 명령이 통과한다.

## 검증 증거

- `scripts/check.sh`
- `cargo test --workspace ir`
