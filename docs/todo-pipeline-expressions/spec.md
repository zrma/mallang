# Spec: pipeline-expressions

## 목표

- Mallang의 함수형 value-first 스타일을 위해 `|>` pipeline call sugar를 추가한다.
- 기존 call semantic/IR/backend를 재사용해 새 runtime primitive 없이 native smoke까지 닫는다.

## 범위

- Parser는 `value |> f(args...)`를 `f(value, args...)` 호출 AST로 낮춘다.
- Pipeline target은 v0에서 direct call form으로 제한한다.
- Piped value는 v0에서 owned first argument로 전달한다.
- Parser/semantic unit test와 `examples/pipeline.mlg` native smoke를 추가한다.
- Borrow-mode first argument pipeline syntax, method pipeline, partial application은 제외한다.

## C-체크리스트

| ID | 상태 | Verify command | 작업 항목 |
| --- | --- | --- | --- |
| C1 | done | `scripts/check.sh` | `|>` pipeline call sugar parser, semantic smoke, native smoke 추가 |

## 완료 기준

- `scripts/check.sh`가 통과한다.
- `examples/pipeline.mlg`가 native binary로 `15`와 `mallang`을 출력한다.
