# Spec: field-borrow-args

## 목표

- `in user.name`과 `mut user.name`처럼 direct local field를 borrow
  argument로 넘기는 v0 규칙을 구현한다.
- same-call borrow conflict check를 root variable 단위에서 field-aware place
  단위로 확장한다.

## 범위

- 허용: direct local variable borrow argument (`in user`, `mut user`).
- 허용: direct local field borrow argument (`in user.name`, `mut user.name`).
- 허용: 같은 root의 disjoint field에 대한 mutable borrow
  (`mut pair.left`, `mut pair.right`).
- 거부: same field 또는 whole-root와 겹치는 exclusive borrow.
- 거부: immutable root binding의 mutable field borrow.
- 거부: nested field borrow argument (`in user.name.value`).
- 제외: nested field assignment.
- 제외: caller-visible mutation을 위한 native by-reference ABI 변경.

## C-체크리스트

| ID | 상태 | Verify command | 작업 항목 |
| --- | --- | --- | --- |
| C1 | done | `cargo test semantic::tests::` | field-aware borrow argument semantic test 추가 |
| C2 | done | `scripts/check.sh` | `examples/field-borrow.mlg` native smoke 추가 |
| C3 | done | `scripts/check.sh` | README/SPEC/ROADMAP/HANDOFF 상태 갱신 |

## 완료 기준

- C-체크리스트가 완료되고 검증 명령이 통과한다.
