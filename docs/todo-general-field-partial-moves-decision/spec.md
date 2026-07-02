# Spec: general-field-partial-moves-decision

## 목표

- v0에서 general field partial move semantics를 열지 않는다.
- Owned slice field take만 v0 field-take 예외로 유지한다.
- Cleanup-capable parent struct가 부분 초기화 상태가 되는 경로를 막는다.

## 결정

- `taken := bag.values`처럼 owned slice field를 가져오는 것은 허용한다.
- `profile := user.profile`처럼 non-slice move-only field를 가져오는 것은 v0에서
  허용하지 않는다.
- General partial moves는 destructuring, field initialization state, cleanup
  suppression, later field reassignment rules를 함께 설계한 뒤 연다.

## 범위

- Semantic regression: non-slice cleanup field move reject.
- SPEC/roadmap/handoff 갱신.

## 제외

- Destructuring syntax.
- General partial-move implementation.
- Field initialization state tracking.
- Backend cleanup suppression for partially moved structs.

## C-체크리스트

| ID | 상태 | Verify command | 작업 항목 |
| --- | --- | --- | --- |
| C1 | done | `cargo test --all-targets partial_move` | semantic partial-move boundary regression |
| C2 | done | `scripts/check.sh` | full native smoke 유지 |
