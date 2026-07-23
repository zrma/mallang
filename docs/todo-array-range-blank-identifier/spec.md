# Spec: array-range-blank-identifier

Status: complete; historical milestone record

## 목표

- Array-only `range` loop에서 Go-like blank identifier `_`를 지원한다.
- `_`는 loop body local binding을 만들지 않는다.
- value binding이 `_`이면 element value copy가 필요 없으므로 non-copy array도
  index-only range가 가능하다.

## 범위

- Parser:
  - 기존 `for i, value := range values` surface를 유지한다.
  - `for _, value := range values`, `for i, _ := range values`,
    `for _, _ := range values`를 parse한다.
- Semantic checker:
  - `_` binding은 local scope에 추가하지 않는다.
  - `index_name == value_name` reject는 둘 다 `_`가 아닌 경우에만 적용한다.
  - value binding이 `_`가 아닐 때만 element type이 `Copy`여야 한다.
  - `_`를 loop body에서 읽으면 일반 unknown variable diagnostic을 유지한다.
- IR/backend:
  - blank index는 backend 내부 loop index temp로만 존재하고 Mallang binding이 아니다.
  - blank value는 C local element copy를 생성하지 않는다.
- 이번 slice에서는 one-variable range, mutable range values, by-reference range,
  slice range는 다루지 않는다.

## C-체크리스트

| ID | 상태 | Verify command | 작업 항목 |
| --- | --- | --- | --- |
| C1 | done | `scripts/check.sh` | parser/semantic support for blank range bindings |
| C2 | done | `scripts/check.sh` | native smoke for blank range bindings |
| C3 | done | `scripts/check.sh` | docs/spec/roadmap/handoff 갱신 |

## 완료 기준

- C-체크리스트가 완료되고 검증 명령이 통과한다.
