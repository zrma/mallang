# Spec: fixed-array-bounds-checks

## 목표

- fixed-size array indexing이 Rust-like safety 목표에 맞게 unchecked C memory
  access로 내려가지 않도록 한다.
- 컴파일 타임에 보이는 out-of-bounds literal index는 `mlg check`에서 막고,
  런타임 값 index는 native backend가 guard를 생성한다.

## 범위

- `values[3]` for `[3]int` 같은 non-negative literal out-of-bounds index reject.
- `values[-1]` 같은 negative literal index reject.
- `values[i]` 같은 non-literal index는 C backend에서 `0 <= i < N` guard 생성.
- guard 실패 시 `stderr`에 Mallang runtime error를 출력하고 `exit(1)`.
- base/index expression은 temp로 평가해 중복 평가와 side effect 제거를 피한다.
- slice `[]T`, borrowed/non-copy indexing, mutable element assignment의 bounds
  semantics는 후속 work로 유지한다.

## C-체크리스트

| ID | 상태 | Verify command | 작업 항목 |
| --- | --- | --- | --- |
| C1 | done | `scripts/check.sh` | literal out-of-bounds index semantic reject 추가 |
| C2 | done | `scripts/check.sh` | native runtime bounds guard codegen 추가 |
| C3 | done | `scripts/check.sh` | dynamic in-bounds index native smoke 유지 |

## 완료 기준

- C-체크리스트가 완료되고 검증 명령이 통과한다.
