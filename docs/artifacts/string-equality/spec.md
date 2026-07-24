# Spec: string-equality

Status: complete; historical milestone record

## 목표

- `string`에 대해 `==`/`!=` equality를 지원한다.
- equality 비교가 non-copy `string` 값을 move하지 않도록 checker와 native backend를 맞춘다.

## 범위

- Semantic checker에서 equality operand를 borrow-use로 검사한다.
- `int`, `bool`, `string` equality만 허용하고 ordering/arithmetic은 기존 제한을 유지한다.
- C backend에서 `string` equality를 `strcmp(...) == 0` / `!= 0`로 낮춘다.
- `examples/string-equality.mlg` native smoke를 추가한다.

## C-체크리스트

| ID | 상태 | Verify command | 작업 항목 |
| --- | --- | --- | --- |
| C1 | done | `scripts/check.sh` | string equality semantic/backend/native smoke 추가 |

## 완료 기준

- C-체크리스트가 완료되고 검증 명령이 통과한다.
