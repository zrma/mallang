# Spec: print-adt-values

## 목표

- C backend에서 `Option` / `Result` 값을 native `print`로 출력한다.
- ADT expression은 한 번만 평가하고, printable payload를 재귀적으로 표시한다.

## 범위

- Top-level `print(value)`에서 `Option[T]`와 `Result[T, E]`를 지원한다.
- 출력 포맷은 `Some(value)`, `None`, `Ok(value)`, `Err(value)`로 고정한다.
- Payload type은 기존 primitive print 대상과 nested printable ADT를 지원한다.
- Struct debug 출력과 사용자 정의 formatter는 제외한다.
- `examples/print-adt.mlg` native smoke로 primitive payload와 nested ADT payload를 검증한다.

## C-체크리스트

| ID | 상태 | Verify command | 작업 항목 |
| --- | --- | --- | --- |
| C1 | done | `scripts/check.sh` | `Option` / `Result` native print와 smoke 추가 |

## 완료 기준

- `scripts/check.sh`가 통과한다.
- `examples/print-adt.mlg` native output이 `Some(7)`, `None`, `Ok(1)`, `Err(bad)`, `Some(Ok(9))`를 출력한다.
