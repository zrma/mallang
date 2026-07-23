# Spec: array-literal-parser

Status: complete; historical milestone record

## 목표

- Fixed-size array literal `[N]T{...}`를 parser AST까지 연결한다.
- Semantic/type checking은 다음 slice로 분리하되 명시적 에러를 남긴다.

## 범위

- Expression 위치에서 `[N]T{...}`를 parse한다.
- Literal type은 기존 `[N]T` `TypeRef` representation을 재사용한다.
- `{}`는 빈 array literal로 허용한다. 길이 일치 검사는 semantic slice에서 처리한다.
- Semantic checker는 이번 slice에서 array literal을 받지 않고
  `fixed-size array literals are parsed but not type-checked yet` 에러를 낸다.
- Array semantic type, move-only ownership rule, IR/backend layout, range loop는 제외한다.

## C-체크리스트

| ID | 상태 | Verify command | 작업 항목 |
| --- | --- | --- | --- |
| C1 | done | `scripts/check.sh` | `[N]T{...}` parser와 AST representation 추가 |
| C2 | done | `scripts/check.sh` | semantic/IR boundary error 추가 |
| C3 | done | `scripts/check.sh` | ROADMAP/HANDOFF 갱신 |

## 완료 기준

- C-체크리스트가 완료되고 검증 명령이 통과한다.
