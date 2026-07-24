# Spec: for-clause-initless

Status: complete; historical milestone record

## 목표

- Go-like initless clause loop `for ; condition; post { ... }`를 parser,
  semantic checker, typed IR, C backend smoke까지 연결한다.

## 범위

- `for ; i < n; i = i + 1 { ... }` 형태를 지원한다.
- condition clause는 필수이며 type은 `bool`이어야 한다.
- post clause는 변수 또는 field assignment 하나만 허용한다.
- init binding이 없으므로 header/body/post는 바깥 binding을 그대로 사용한다.
- `continue`는 post clause를 실행한 뒤 다음 iteration으로 간다.
- empty condition, empty post, `range` loop는 이번 slice에서 제외한다.

## C-체크리스트

| ID | 상태 | Verify command | 작업 항목 |
| --- | --- | --- | --- |
| C1 | done | `scripts/check.sh` | Parser가 `for ; condition; post`를 clause loop로 인식 |
| C2 | done | `scripts/check.sh` | Semantic/IR/backend optional init 경로 검증 |
| C3 | done | `scripts/check.sh` | initless native smoke와 문서 갱신 |

## 완료 기준

- C-체크리스트가 완료되고 검증 명령이 통과한다.
