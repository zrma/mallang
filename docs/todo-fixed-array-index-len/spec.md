# Spec: fixed-array-index-len

Status: complete; historical milestone record

## 목표

- fixed-size array의 직접 index access와 `len` built-in을 compiler pipeline에
  추가한다.
- array-only `range` 이후에도 source array를 move하지 않고 재사용할 수 있음을
  native smoke로 검증한다.

## 범위

- `values[i]` expression parser와 AST/IR 노드를 추가한다.
- index expression은 base가 fixed-size array이고 index가 `int`일 때만 허용한다.
- v0 값 반환은 `Copy` element에만 허용한다.
- `len(values)`는 fixed-size array에 대해 `int`를 반환하고 array를 move하지 않는다.
- C backend는 array struct wrapper의 `.mlg_data[index]`와 type-level length를
  사용한다.
- `len` 인자 표현식은 side effect를 제거하지 않도록 C prelude에서 평가한다.
- borrowed/non-copy indexing, mutable element assignment, runtime bounds checks,
  slice `[]T`, append/growth는 후속 work로 남긴다.

## C-체크리스트

| ID | 상태 | Verify command | 작업 항목 |
| --- | --- | --- | --- |
| C1 | done | `scripts/check.sh` | parser/semantic/IR/backend에 fixed array indexing 추가 |
| C2 | done | `scripts/check.sh` | `len(values)` fixed-size array built-in 추가 |
| C3 | done | `scripts/check.sh` | `examples/arrays.mlg` native smoke에서 indexing, `len`, source reuse 검증 |

## 완료 기준

- C-체크리스트가 완료되고 검증 명령이 통과한다.
