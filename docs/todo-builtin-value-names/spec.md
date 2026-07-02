# Spec: builtin-value-names

## 목표

- Built-in value namespace를 semantic checker에서 예약한다.
- User-defined value binding이 `print`, `len`, `Some`, `None`, `Ok`, `Err`와
  충돌하지 않게 한다.

## 범위

- 거부:
  - global function 이름
  - parameter와 receiver 이름
  - local binding과 `for` init binding 이름
  - range index/value binding 이름
  - `match` payload binding 이름
- 허용:
  - struct field 이름
  - concrete struct method 이름
- Type namespace의 built-in 이름(`int`, `bool`, `string`, `unit`, `Option`,
  `Result`)은 기존 `is_builtin_type_name` 규칙을 유지한다.

## C-체크리스트

| ID | 상태 | Verify command | 작업 항목 |
| --- | --- | --- | --- |
| C1 | done | `cargo test --workspace rejects_builtin_value` | semantic regression 추가 |
| C2 | done | `scripts/check.sh` | `mlg check` failure smoke 추가 |

## 완료 기준

- C-체크리스트가 완료되고 검증 명령이 통과한다.
