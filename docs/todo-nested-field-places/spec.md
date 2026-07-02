# Spec: nested-field-places

## 목표

- `user.name.value`처럼 local binding에서 시작하는 nested field path를
  assignment와 borrow argument에서 지원한다.
- same-call borrow conflict check를 nested field path prefix 기반으로 확장한다.

## 범위

- 허용: nested field assignment (`user.name.value = "lee"`).
- 허용: nested field read borrow (`con user.name.value`).
- 허용: nested field mutable borrow (`mut user.name.value`) when the root
  binding is `mut`.
- 허용: 서로 prefix 관계가 아닌 disjoint nested field mutable borrows.
- 거부: root binding이 immutable인 nested field assignment/mutable borrow.
- 거부: non-struct field를 통과하는 nested field path.
- 거부: 같은 field path 또는 parent/child prefix 관계와 겹치는 exclusive
  borrow.
- 제외: caller-visible mutation을 위한 native by-reference ABI 변경.

## C-체크리스트

| ID | 상태 | Verify command | 작업 항목 |
| --- | --- | --- | --- |
| C1 | done | `cargo test semantic::tests::` | nested field assignment/borrow semantic test 추가 |
| C2 | done | `scripts/check.sh` | `examples/nested-fields.mlg` native smoke 추가 |
| C3 | done | `scripts/check.sh` | README/SPEC/ROADMAP/HANDOFF 상태 갱신 |

## 완료 기준

- C-체크리스트가 완료되고 검증 명령이 통과한다.
