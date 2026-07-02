# Spec: for-clause-loop

## 목표

- Go-like `for init; condition; post` loop를 parser, semantic checker, typed IR,
  C backend, native smoke까지 연결한다.

## 범위

- `for mut i := 0; i < n; i = i + 1 { ... }` 형태를 지원한다.
- init clause는 `name := expr` 또는 `mut name := expr`만 허용한다.
- condition clause는 필수이며 type은 `bool`이어야 한다.
- post clause는 변수 또는 field assignment 하나만 허용한다.
- init binding은 loop header/body/post 안에서만 보이고 loop 밖으로 새지 않는다.
- `continue`는 C `for` semantics처럼 post clause를 실행한 뒤 다음 iteration으로 간다.
- `range`, initless clause loop, empty condition, post declaration, header prelude
  codegen은 이번 slice에서 제외한다.

## C-체크리스트

| ID | 상태 | Verify command | 작업 항목 |
| --- | --- | --- | --- |
| C1 | done | `scripts/check.sh` | Parser/semantic/IR/backend에 clause loop 추가 |
| C2 | done | `scripts/check.sh` | `continue` + post semantics native smoke 추가 |
| C3 | done | `scripts/check.sh` | SPEC/ROADMAP/README/HANDOFF 갱신 |

## 완료 기준

- C-체크리스트가 완료되고 검증 명령이 통과한다.
