# Spec: entrypoint-signature

## 목표

- v0 native backend wrapper와 semantic checker가 같은 entrypoint contract를
  공유하도록 `func main()` signature를 고정한다.
- `main`은 program entrypoint 이름으로 예약하고 receiver-qualified method
  이름으로도 사용하지 않는다.

## 범위

- Semantic checker에서 `main` receiver, parameter, return type을 reject한다.
- Backend는 semantic checker가 보장한 no-argument entrypoint만 C `main(void)`로
  lowering한다.
- `mlg check` failure smoke로 invalid entrypoint signature를 고정한다.

## C-체크리스트

| ID | 상태 | Verify command | 작업 항목 |
| --- | --- | --- | --- |
| C1 | done | `cargo test --workspace rejects_main` | semantic regression coverage |
| C2 | done | `scripts/check.sh` | invalid `main` parameter smoke |
| C3 | done | `SPEC.md` / `README.md` | user-facing entrypoint contract |

## 완료 기준

- C-체크리스트가 완료되고 검증 명령이 통과한다.
