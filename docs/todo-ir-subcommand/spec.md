# Spec: ir-subcommand

## 목표

- `mlg ir <source-file>` debug subcommand를 추가해 checked AST를 typed IR로 낮춘 결과를 바로 볼 수 있게 한다.

## 범위

- `parse -> check -> lower` 순서로 실행한다.
- `IrProgram`을 Rust pretty debug format으로 출력한다.
- IR 안정 포맷이나 machine-readable output은 범위 밖이다.

## C-체크리스트

| ID | 상태 | Verify command | 작업 항목 |
| --- | --- | --- | --- |
| C1 | done | `scripts/check.sh` | `mlg ir` CLI subcommand를 추가한다. |
| C2 | done | `scripts/check.sh` | `examples/adt.mlg` IR smoke를 검증 스크립트에 추가한다. |
| C3 | done | `scripts/check.sh` | README/HANDOFF에 subcommand를 문서화한다. |

## 완료 기준

- C-체크리스트가 완료되고 검증 명령이 통과한다.
