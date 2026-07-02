# Spec: generic-type-refs

## 목표

- `Option[T]`와 `Result[T, E]` 구현의 첫 단계로 generic type reference를 AST까지 파싱한다.
- type checker는 아직 ADT를 허용하지 않고, planned feature임을 명확히 진단한다.

## 범위

- `TypeRef`에 nested type argument를 보존한다.
- parser가 `Name[T]`, `Name[T, E]`, nested type ref를 읽는다.
- primitive type에 잘못 붙은 type argument와 `Option`/`Result` arity 오류를 진단한다.
- `Option`/`Result` constructor와 `match` 구현은 범위 밖이다.

## C-체크리스트

| ID | 상태 | Verify command | 작업 항목 |
| --- | --- | --- | --- |
| C1 | done | `scripts/check.sh` | generic `TypeRef` AST/parser를 추가한다. |
| C2 | done | `scripts/check.sh` | semantic checker가 planned ADT type diagnostic을 제공한다. |
| C3 | done | `scripts/check.sh` | handoff/roadmap의 다음 boundary를 constructor type checking으로 이동한다. |

## 완료 기준

- C-체크리스트가 완료되고 검증 명령이 통과한다.
