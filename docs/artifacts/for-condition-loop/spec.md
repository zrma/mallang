# Spec: for-condition-loop

Status: complete; historical milestone record

## 목표

- Go-like control flow의 기본으로 condition-only `for` statement를 추가한다.
- Native backend에서는 C `while` loop로 낮춘다.

## 범위

- 문법은 `for condition { ... }`만 지원한다.
- Condition은 `bool`이어야 한다.
- Loop body binding은 body 밖으로 leak되지 않는다.
- Loop body에서 outer non-copy value를 move하면 loop 이후 사용할 수 없게 처리한다.
- `break`, `continue`, `range`, Go three-clause `for init; condition; post`는 제외한다.
- `examples/for-loop.mlg` native smoke로 condition side effect가 매 iteration 실행되는지 검증한다.

## C-체크리스트

| ID | 상태 | Verify command | 작업 항목 |
| --- | --- | --- | --- |
| C1 | done | `scripts/check.sh` | condition-only `for` parser, semantic, IR, C backend, native smoke 추가 |

## 완료 기준

- `scripts/check.sh`가 통과한다.
- `examples/for-loop.mlg` native output이 `1`, `2`, `3`, `4`를 순서대로 출력한다.
