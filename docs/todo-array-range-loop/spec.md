# Spec: array-range-loop

Status: complete; historical milestone record

## 목표

- Array-only `for i, value := range values { ... }` loop를 parser, semantic,
  IR, C backend까지 연결한다.

## 범위

- Two-variable range form만 지원한다.
- Range source는 fixed-size array여야 한다.
- Index binding은 immutable `int`로 body에만 scope를 가진다.
- Value binding은 immutable element type으로 body에만 scope를 가진다.
- v0에서는 value binding이 element copy이므로 element type이 `Copy`여야 한다.
- Range source는 move하지 않고 읽으므로 loop 이후에도 사용할 수 있다.
- Native backend는 source를 temp에 한 번 담고 C `for` loop로 순회한다.
- Blank identifier, one-variable range, mutable/by-reference range, slices는 제외한다.

## C-체크리스트

| ID | 상태 | Verify command | 작업 항목 |
| --- | --- | --- | --- |
| C1 | done | `scripts/check.sh` | range keyword/parser/AST 추가 |
| C2 | done | `scripts/check.sh` | array-only semantic과 Copy element rule 추가 |
| C3 | done | `scripts/check.sh` | typed IR와 C backend range loop lowering 추가 |
| C4 | done | `scripts/check.sh` | `examples/arrays.mlg` native output smoke 갱신 |

## 완료 기준

- C-체크리스트가 완료되고 검증 명령이 통과한다.
