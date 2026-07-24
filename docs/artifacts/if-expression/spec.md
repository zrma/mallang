# Spec: if-expression

Status: complete; historical milestone record

## 목표

- SPEC에 이미 정의된 `if` expression의 첫 구현을 추가한다.
- v0에서는 `condition`이 `bool`이고 양 branch가 같은 non-`unit` 값을 내는 expression 형태만 지원한다.

## 범위

- 지원 syntax: `if <bool-expr> { <expr> } else { <expr> }`.
- `if` expression에는 `else`가 필수다.
- branch body는 이번 slice에서 단일 expression만 허용한다. statement block과 statement-form `if`는 범위 밖이다.
- typed IR가 branch expression type을 보존하고 C backend가 ternary expression으로 emit한다.

## C-체크리스트

| ID | 상태 | Verify command | 작업 항목 |
| --- | --- | --- | --- |
| C1 | done | `scripts/check.sh` | parser/semantic/IR/backend에 `if` expression을 연결한다. |
| C2 | done | `scripts/check.sh` | native smoke 예제로 `if` expression 결과를 검증한다. |

## 완료 기준

- C-체크리스트가 완료되고 검증 명령이 통과한다.
