# Spec: print-struct-values

## 목표

- C backend에서 struct 값을 native `print`로 출력한다.
- Nested struct와 ADT field를 재귀적으로 표시한다.

## 범위

- 출력 포맷은 `Type{field: value, ...}`로 고정한다.
- Field 출력 순서는 struct 선언 순서를 따른다.
- Field type은 기존 printable primitive, printable ADT, printable struct를 지원한다.
- Recursive by-value structs는 v0에서 이미 지원하지 않으므로 별도 cycle guard는 추가하지 않는다.
- `examples/print-struct.mlg` native smoke로 nested struct와 ADT field 출력을 검증한다.

## C-체크리스트

| ID | 상태 | Verify command | 작업 항목 |
| --- | --- | --- | --- |
| C1 | done | `scripts/check.sh` | struct native print와 smoke 추가 |

## 완료 기준

- `scripts/check.sh`가 통과한다.
- `examples/print-struct.mlg` native output이 `User{name: kim, age: 30, active: true, profile: Profile{display: neo}, status: Some(7)}`를 출력한다.
