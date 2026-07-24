# Spec: array-semantics

Status: complete; historical milestone record

## 목표

- Fixed-size array type과 `[N]T{...}` literal을 semantic checker에 연결한다.
- v0 결정대로 fixed-size array 값은 element type과 관계없이 move-only로 취급한다.

## 범위

- `Type::Array { len, element }` semantic type을 추가한다.
- `[N]T` type reference를 semantic type으로 변환한다.
- Array literal 길이가 `N`과 정확히 일치하는지 검사한다.
- Array literal element가 declared element type과 일치하는지 검사한다.
- Expected type context와 array literal declared type이 일치하는지 검사한다.
- Array 값의 use-after-move를 기존 ownership-lite 경로로 검증한다.
- IR/backend는 이번 slice에서 array lowering/layout을 구현하지 않고 명시적 boundary error를 유지한다.

## C-체크리스트

| ID | 상태 | Verify command | 작업 항목 |
| --- | --- | --- | --- |
| C1 | done | `scripts/check.sh` | semantic `Type::Array`와 `[N]T` 변환 추가 |
| C2 | done | `scripts/check.sh` | array literal 길이/element/expected type 검사 추가 |
| C3 | done | `scripts/check.sh` | array move-only ownership test 추가 |
| C4 | done | `scripts/check.sh` | IR/backend boundary와 check-only example smoke 추가 |

## 완료 기준

- C-체크리스트가 완료되고 검증 명령이 통과한다.
