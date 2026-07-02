# Spec: top-level-declaration-namespace

## 목표

- v0의 top-level declaration namespace를 semantic checker에서 고정한다.
- `type Name struct`와 non-method `func Name(...)`가 같은 이름을 공유하지
  못하게 한다.

## 범위

- 거부:
  - top-level struct와 non-method function의 같은 이름
  - top-level struct 이름이 built-in value name과 충돌하는 경우
  - top-level non-method function 이름이 built-in type name과 충돌하는 경우
- 유지:
  - duplicate struct/function/method diagnostics
  - concrete method 이름은 receiver-qualified namespace
  - struct field 이름은 struct-local namespace

## C-체크리스트

| ID | 상태 | Verify command | 작업 항목 |
| --- | --- | --- | --- |
| C1 | done | `cargo test --workspace top_level` | semantic regression 추가 |
| C2 | done | `scripts/check.sh` | `mlg check` failure smoke 추가 |

## 완료 기준

- C-체크리스트가 완료되고 검증 명령이 통과한다.
