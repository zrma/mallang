# Spec: loop-control

## 목표

- Condition-only `for` loop에 필요한 최소 loop control statement를 추가한다.
- `break`와 `continue`를 semantic/IR/C backend/native smoke까지 연결한다.

## 범위

- `break`는 nearest enclosing loop를 종료한다.
- `continue`는 nearest enclosing loop의 다음 iteration으로 이동한다.
- Loop 밖의 `break`와 `continue`는 semantic error로 거부한다.
- `examples/loop-control.mlg` native smoke로 skip과 early exit를 검증한다.
- Labeled break/continue, `defer`, `finally`류 control cleanup은 제외한다.

## C-체크리스트

| ID | 상태 | Verify command | 작업 항목 |
| --- | --- | --- | --- |
| C1 | done | `scripts/check.sh` | `break` / `continue` parser, semantic, IR, C backend, native smoke 추가 |

## 완료 기준

- `scripts/check.sh`가 통과한다.
- `examples/loop-control.mlg` native output이 `1`, `3`, `4`, `5`를 순서대로 출력한다.
