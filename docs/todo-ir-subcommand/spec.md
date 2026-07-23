# Spec: ir-subcommand

Status: complete; historical milestone record

## 목표

- `mlg ir <input>` debug subcommand를 추가해 checked AST를 typed IR로 낮춘
  결과를 바로 볼 수 있게 한다.

## 범위

- `parse -> check -> lower` 순서로 실행한다.
- standalone source와 project input을 같은 명령으로 지원한다.
- B5 이전 Rust Stage0은 `IrProgram` pretty debug를 출력했다. 그 형식은 안정
  포맷이 아니었고 Rust 구현 세부를 노출했으므로 B5 P179b2b3에서 기존
  self-hosting differential의 deterministic `IR|...` 정규형으로 교체한다.
- Stage0와 self compiler는 같은 byte output을 내며, 내부 compiler protocol
  version과 함께 진화한다. 별도의 장기 안정 machine-readable API는 범위 밖이다.

## C-체크리스트

| ID | 상태 | Verify command | 작업 항목 |
| --- | --- | --- | --- |
| C1 | done | `scripts/check.sh` | `mlg ir` CLI subcommand를 추가한다. |
| C2 | done | `scripts/check.sh` | `examples/adt.mlg` IR smoke를 검증 스크립트에 추가한다. |
| C3 | done | `scripts/check.sh` | README/HANDOFF에 subcommand를 문서화한다. |
| C4 | done | `scripts/check-self-hosting-default-compiler.sh` | standalone/project Stage0/self normalized IR parity를 검증한다. |

## 완료 기준

- C-체크리스트가 완료되고 검증 명령이 통과한다.
