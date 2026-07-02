# Spec: array-ir-backend

## 목표

- Fixed-size array typed IR와 C struct-wrapper layout을 추가한다.
- `[N]T{...}` array literal을 native backend까지 연결한다.

## 범위

- IR `Type::Array`와 `IrExprKind::ArrayLiteral` lowering을 추가한다.
- C backend는 array를 struct wrapper로 emit한다.
- Non-empty array는 `mlg_data[N]` field를 가진다.
- `[0]T`는 표준 C zero-length array를 피하기 위해 dummy field를 가진다.
- Array value는 local binding, owned parameter, return value의 기존 value pipeline을 사용한다.
- Indexing, mutation, `len`, `range`, array printing은 제외한다.

## C-체크리스트

| ID | 상태 | Verify command | 작업 항목 |
| --- | --- | --- | --- |
| C1 | done | `scripts/check.sh` | array literal typed IR lowering 추가 |
| C2 | done | `scripts/check.sh` | C struct-wrapper typedef와 literal initializer 추가 |
| C3 | done | `scripts/check.sh` | `examples/arrays.mlg` ir/build native smoke 추가 |

## 완료 기준

- C-체크리스트가 완료되고 검증 명령이 통과한다.
