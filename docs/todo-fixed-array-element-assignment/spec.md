# Spec: fixed-array-element-assignment

## 목표

- fixed-size array의 `Copy` element를 checked index를 통해 갱신할 수 있게 한다.
- Go-like `values[i] = expr` syntax를 유지하면서 mutability는 Mallang binding
  규칙(`mut values := ...` 또는 `mut` parameter)에 따른다.

## 범위

- statement form `values[i] = expr` parser 지원.
- assignment target은 v0에서 direct mutable fixed-size array local 또는 `mut`
  array parameter만 허용한다.
- element type은 `Copy`여야 한다.
- RHS는 element type과 일치해야 한다.
- literal out-of-bounds index는 `mlg check`에서 reject한다.
- non-literal index는 native backend가 bounds guard를 생성한다.
- index는 RHS보다 먼저 평가하고 bounds-check한다.
- field path array assignment, for-post index assignment, borrowed/non-copy
  element assignment, slice element assignment는 후속 work로 남긴다.

## C-체크리스트

| ID | 상태 | Verify command | 작업 항목 |
| --- | --- | --- | --- |
| C1 | done | `scripts/check.sh` | parser/semantic에 fixed array element assignment 추가 |
| C2 | done | `scripts/check.sh` | typed IR와 C backend에 checked element assignment 추가 |
| C3 | done | `scripts/check.sh` | `examples/arrays.mlg` native smoke에서 assignment 이후 range/index/len 검증 |

## 완료 기준

- C-체크리스트가 완료되고 검증 명령이 통과한다.
