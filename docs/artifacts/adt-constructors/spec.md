# Spec: adt-constructors

Status: complete; historical milestone record

## 목표

- `Option[T]` / `Result[T, E]` 타입을 semantic checker의 실제 타입 모델에 추가한다.
- `Some` / `None` / `Ok` / `Err` constructor를 `mlg check`에서 타입 검사한다.

## 범위

- `Option[T]`는 `Some(value)`와 `None`으로 생성한다.
- `Result[T, E]`는 `Ok(value)`와 `Err(error)`로 생성한다.
- `None`, `Ok`, `Err`는 return type, parameter type, assignment target, 또는 `if` expression의 expected type context가 필요하다.
- `Some(value)`는 payload type으로 `Option[payload]`를 추론할 수 있다.
- tagged typed IR와 C backend layout은 다음 work unit으로 분리한다. 이번 slice에서 `mlg build`는 ADT lowering/codegen 미구현 에러를 낼 수 있다.

## C-체크리스트

| ID | 상태 | Verify command | 작업 항목 |
| --- | --- | --- | --- |
| C1 | done | `scripts/check.sh` | semantic `Type` 모델에 `Option` / `Result`를 추가한다. |
| C2 | done | `scripts/check.sh` | `Some` / `None` / `Ok` / `Err` constructor type checking을 추가한다. |
| C3 | done | `scripts/check.sh` | ADT lowering/codegen 미구현 경계를 명확한 diagnostic으로 남긴다. |
| C4 | done | `scripts/check.sh` | handoff/roadmap의 다음 boundary를 exhaustive `match` 또는 tagged IR로 이동한다. |

## 완료 기준

- C-체크리스트가 완료되고 검증 명령이 통과한다.
