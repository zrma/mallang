# Spec: append-builtin-reservation

Status: complete; historical milestone record

## Goal

- Slice growth surface를 열기 전에 `append` 이름을 built-in value namespace에 예약한다.
- v0에서 사용자 선언이 future `append` built-in과 충돌하지 않도록 한다.

## Scope

- Semantic checker의 built-in value name set에 `append`를 추가한다.
- Top-level function, local binding, parameter/range/match payload binding은 기존
  built-in value reservation 규칙을 그대로 따른다.
- `append` implementation, slice values, growth ABI는 아직 도입하지 않는다.

## Acceptance

| ID | Status | Evidence |
| --- | --- | --- |
| C1 | done | `cargo test --workspace rejects_builtin_value` |
| C2 | done | `scripts/check.sh` built-in value failure smoke |
| C3 | done | `SPEC.md` reserved built-in value list includes `append` |
