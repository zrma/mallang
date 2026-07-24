# Spec: cleanup-assignment-rhs-before-drop

Status: complete; historical milestone record

## 목표

- Cleanup overwrite assignment에서 RHS를 먼저 평가한 뒤 old place를 drop하고
  temp를 assignment하는 IR 순서를 보장한다.
- Future slice `values = make()` / field overwrite / array element overwrite에서
  old value가 RHS evaluation 전에 사라지지 않게 한다.

## 범위

- Cleanup type local reassignment는 RHS를 internal temp `let`으로 먼저 평가한다.
- Cleanup type field assignment는 RHS temp, old field drop, field assignment 순서로
  IR을 생성한다.
- Cleanup type fixed-array element assignment는 RHS temp, old element drop, element
  assignment 순서로 IR을 생성한다.
- Source-level slice surface는 계속 reserved 상태로 둔다.

## 제외

- Expression-form `if`/`match` branch cleanup normalization.
- Struct-wide drop helper for user structs.
- Partial move/destructuring.
- Source-level `drop(value)` syntax.
- Slice literal, `len(slice)`, `slice[i]`, `append` 구현.

## C-체크리스트

| ID | 상태 | Verify command | 작업 항목 |
| --- | --- | --- | --- |
| C1 | done | `cargo test --workspace cleanup_rhs` | local reassignment RHS-before-drop regression |
| C2 | done | `cargo test --workspace cleanup_field` | field overwrite RHS-before-drop regression |
| C3 | done | `cargo test --workspace cleanup_array_element` | array element overwrite RHS-before-drop regression |
| C4 | done | `scripts/check.sh` | existing native surface and cleanup regressions 유지 |

## 완료 기준

- C-체크리스트가 완료되고 검증 명령이 통과한다.
