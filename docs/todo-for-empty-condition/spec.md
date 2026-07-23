# Spec: for-empty-condition

Status: complete; historical milestone record

## 목표

- Go-like conditionless loop `for { ... }`와 empty-condition clause loop
  `for ; ; post { ... }`를 parser, semantic checker, typed IR, C backend,
  native smoke까지 연결한다.

## 범위

- `for { ... }`는 conditionless infinite loop로 해석한다.
- `for ; ; i = i + 1 { ... }`는 C/Go처럼 condition을 비운 clause loop로
  해석한다.
- condition이 있으면 기존처럼 `bool`이어야 한다.
- `break` / `continue` semantics는 기존 loop control 규칙을 따른다.
- `continue`는 clause loop의 post를 실행한 뒤 다음 iteration으로 간다.
- `range` loop, post declaration, array/slice surface는 이번 slice에서 제외한다.

## C-체크리스트

| ID | 상태 | Verify command | 작업 항목 |
| --- | --- | --- | --- |
| C1 | done | `scripts/check.sh` | AST/parser가 optional for condition을 표현 |
| C2 | done | `scripts/check.sh` | Semantic/IR/backend가 conditionless loop를 처리 |
| C3 | done | `scripts/check.sh` | Native smoke와 SPEC/README/HANDOFF/ROADMAP 갱신 |

## 완료 기준

- C-체크리스트가 완료되고 검증 명령이 통과한다.
