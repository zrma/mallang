# Spec: array-range-one-variable

Status: complete; historical milestone record

## 목표

- Go-like one-variable array range syntax를 지원한다.
- `for i := range values`는 index-only range로 동작하며 value binding을 만들지
  않는다.
- element value copy가 없으므로 non-copy element array도 index-only range가
  가능하다.

## 범위

- Parser:
  - `for i := range values { ... }`를 parse한다.
  - `for _ := range values { ... }`도 parse하되 loop body local binding을
    만들지 않는다.
  - AST/IR에서는 기존 blank value lowering을 재사용해 `value_name = "_"`로
    표현한다.
- Semantic checker:
  - one-variable range의 binding은 immutable `int` index binding이다.
  - binding이 `_`이면 local scope에 추가하지 않는다.
  - element type `Copy` requirement는 적용하지 않는다.
- Backend:
  - one-variable range는 value local/copy 없이 C loop만 생성한다.
- 이번 slice에서는 slice range, mutable range values, by-reference range,
  map/string range는 다루지 않는다.

## C-체크리스트

| ID | 상태 | Verify command | 작업 항목 |
| --- | --- | --- | --- |
| C1 | done | `scripts/check.sh` | parser/semantic support for one-variable range |
| C2 | done | `scripts/check.sh` | native smoke for index-only range over non-copy arrays |
| C3 | done | `scripts/check.sh` | docs/spec/roadmap/handoff 갱신 |

## 완료 기준

- C-체크리스트가 완료되고 검증 명령이 통과한다.
