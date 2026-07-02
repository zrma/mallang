# Spec: match-scrutinee-temp

## 목표

- native C backend에서 `match` scrutinee가 local variable이 아닌 expression이어도
  한 번만 평가되도록 temp prelude를 생성한다.

## 범위

- 지원 예: `match maybe(false) { ... }`, `print(match maybe(false) { ... })`
- 기존 local-variable scrutinee 경로는 유지한다.
- temp 이름은 source span 기반의 backend-internal C local로 생성한다.
- `match` arm expression 자체를 statement block으로 낮추는 일반화는 이번 범위 밖이다.
- IR 구조 변경 없이 C backend statement-context emission에서 처리한다.

## C-체크리스트

| ID | 상태 | Verify command | 작업 항목 |
| --- | --- | --- | --- |
| C1 | done | `cargo test backend::tests::generates_temp_for_non_local_match_scrutinee` | non-local match scrutinee temp codegen 추가 |
| C2 | done | `scripts/check.sh` | native smoke `examples/match-temp.mlg` 추가 |
| C3 | done | `scripts/check.sh` | README/SPEC/HANDOFF/roadmap 갱신 |

## 완료 기준

- C-체크리스트가 완료되고 검증 명령이 통과한다.
