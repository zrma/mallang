# Spec: slice-surface-decision

Status: complete; historical milestone record

## 목표

- array-only `range` 이후의 다음 array/slice 작업 경계를 고정한다.
- fixed-size array indexing과 `len`을 먼저 구현하고, full slice surface는
  ownership 결정이 필요한 별도 work item으로 보류한다.

## 범위

- `values[i]`는 fixed-size array `[N]T`에서 `int` index로 접근하는 표현식이다.
- v0 indexing 값 반환은 `Copy` element에만 허용한다.
- `len(values)`는 fixed-size array에 대해 `int`를 반환하는 read-only built-in이다.
- `len(values)`와 range source는 array를 move하지 않는다.
- `[]T`, append/growth, mutable range values, borrowed indexing, non-copy element
  access는 이 slice에서 설계하지 않는다.
- 다음 구현 slice는 parser/semantic/backend를 포함하는 fixed-size array
  indexing + `len`으로 잡는다.

## C-체크리스트

| ID | 상태 | Verify command | 작업 항목 |
| --- | --- | --- | --- |
| C1 | done | `scripts/check.sh` | SPEC/roadmap/handoff에 fixed array indexing + `len` 선행 결정을 반영 |

## 완료 기준

- C-체크리스트가 완료되고 검증 명령이 통과한다.
