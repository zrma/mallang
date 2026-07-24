# Spec: mut-parameter-abi

Status: complete; historical milestone record

## 목표

- `con`/`mut` parameter를 native backend에서 hidden-reference ABI로 낮춘다.
- `mut` parameter에 대한 assignment와 field assignment가 caller의 local/field path에 반영되도록 한다.

## 범위

- C backend function prototype에서 `con T`는 read-only pointer, `mut T`는 mutable pointer로 선언한다.
- C backend call site에서 `con`/`mut` argument는 local 또는 field place의 주소를 넘긴다.
- 함수 body에서는 borrowed parameter를 일반 Mallang value syntax로 읽되, `mut` assignment는 pointer dereference write로 생성한다.
- parser, semantic checker, typed IR shape는 기존 `ParamMode` / `ArgMode` 보존 구조를 그대로 사용한다.

## C-체크리스트

| ID | 상태 | Verify command | 작업 항목 |
| --- | --- | --- | --- |
| C1 | done | `scripts/check.sh` | hidden-reference C ABI 구현 |
| C2 | done | `scripts/check.sh` | caller-visible `mut` parameter native smoke 추가 |

## 완료 기준

- C-체크리스트가 완료되고 검증 명령이 통과한다.
