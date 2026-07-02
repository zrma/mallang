# Spec: slice-type-surface

## 목표

- Go-like slice type syntax `[]T`를 parser surface에 추가한다.
- Mallang v0가 아직 slice ownership/native ABI를 결정하지 않았다는 점을
  semantic checker에서 명시적인 reserved-feature diagnostic으로 고정한다.

## 범위

- Parser:
  - function parameter, return type, struct field, generic type argument 안에서
    `[]T` type reference를 parse한다.
  - `[]T`는 fixed-size array `[N]T`와 구분되는 slice type reference로 AST에 남긴다.
- Semantic checker:
  - `[]T`는 element type과 무관하게 v0 reserved syntax로 reject한다.
  - 기존 fixed-size array `[N]T` 동작은 유지한다.
- IR/backend:
  - semantic이 `[]T`를 막으므로 정상 lowering/codegen surface는 추가하지 않는다.
  - 방어적 IR error는 남긴다.
- 이번 slice에서는 slice literals, append/growth, range over slices, borrowed slice views,
  slice native ABI는 다루지 않는다.

## C-체크리스트

| ID | 상태 | Verify command | 작업 항목 |
| --- | --- | --- | --- |
| C1 | done | `scripts/check.sh` | parser support for `[]T` type refs |
| C2 | done | `scripts/check.sh` | semantic reserved diagnostic for `[]T` |
| C3 | done | `scripts/check.sh` | docs/spec/roadmap/handoff 갱신 |

## 완료 기준

- C-체크리스트가 완료되고 검증 명령이 통과한다.
