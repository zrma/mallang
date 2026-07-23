# Spec: tagged-adt-backend

Status: complete; historical milestone record

## 목표

- `Option[T]` / `Result[T, E]`를 typed IR와 C backend까지 낮춰 native build 경로에 연결한다.
- `examples/adt.mlg`를 `mlg build`로 빌드하고 실행할 수 있게 한다.

## 범위

- IR에 ADT constructor와 `match` expression을 추가한다.
- C backend는 concrete ADT type별 tagged struct typedef를 생성한다.
- Constructor codegen은 C compound literal을 사용한다.
- `match` codegen은 v0에서 direct local variable scrutinee만 지원한다.
- Nested ADT layout, arbitrary expression scrutinee temp lowering, mutation through ADT payload는 범위 밖이다.

## C-체크리스트

| ID | 상태 | Verify command | 작업 항목 |
| --- | --- | --- | --- |
| C1 | done | `scripts/check.sh` | ADT constructor와 `match`를 typed IR로 lowering한다. |
| C2 | done | `scripts/check.sh` | C backend에 concrete `Option` / `Result` tagged layout을 추가한다. |
| C3 | done | `scripts/check.sh` | `examples/adt.mlg` native smoke를 추가한다. |
| C4 | done | `scripts/check.sh` | handoff/roadmap을 다음 boundary로 갱신한다. |

## 완료 기준

- C-체크리스트가 완료되고 검증 명령이 통과한다.
