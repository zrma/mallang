# Spec: array-type-parser

Status: complete; historical milestone record

## 목표

- Fixed-size array type reference `[N]T`를 parser AST까지 연결한다.
- Semantic/type checking은 다음 slice로 분리하되 명시적 에러를 남긴다.

## 범위

- Field, parameter, return type 위치에서 `[N]T`를 parse한다.
- Generic type argument 위치에서도 `[N]T`를 parse한다.
- `N`은 integer literal이며 `usize` 범위를 벗어나면 parse error로 거부한다.
- AST `TypeRef`는 array type을 `name = "Array"`, `array_len = Some(N)`,
  `args = [element]`로 표현한다.
- Semantic checker는 이번 slice에서 array type을 받지 않고
  `fixed-size array types are parsed but not type-checked yet` 에러를 낸다.
- Array literal `[N]T{...}`, array semantic type, IR/backend layout은 제외한다.

## C-체크리스트

| ID | 상태 | Verify command | 작업 항목 |
| --- | --- | --- | --- |
| C1 | done | `scripts/check.sh` | `[N]T` parser와 TypeRef representation 추가 |
| C2 | done | `scripts/check.sh` | semantic boundary error 추가 |
| C3 | done | `scripts/check.sh` | ROADMAP/HANDOFF 갱신 |

## 완료 기준

- C-체크리스트가 완료되고 검증 명령이 통과한다.
