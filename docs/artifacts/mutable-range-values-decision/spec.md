# Spec: mutable-range-values-decision

Status: complete; historical milestone record

## 목표

- v0에서 mutable range value syntax를 열지 않는다.
- 기존 `for i, value := range values`는 immutable copied value binding으로 유지한다.
- by-reference range iteration과 element mutation은 별도 설계가 필요하다는 경계를 고정한다.

## 결정

- `for i, mut value := range values`는 v0 syntax가 아니다.
- `for i, value := range values`의 `value`는 immutable loop-local binding이다.
- Range element mutation은 이미 열린 indexed assignment surface, 예:
  `values[i] = expr`, 를 사용한다.
- 후속 mutable range design은 다음 중 하나를 명시적으로 선택해야 한다.
  - mutable copied loop-local value
  - `mut` element borrow
  - indexed assignment sugar

## 범위

- Parser regression: mutable range value binding syntax reject.
- Semantic regression: range value binding reassignment reject.
- SPEC/roadmap/handoff 갱신.

## 제외

- Mutable range binding implementation.
- By-reference range iteration.
- Statement-spanning borrow lifetime.
- IR/backend range representation 변경.

## C-체크리스트

| ID | 상태 | Verify command | 작업 항목 |
| --- | --- | --- | --- |
| C1 | done | `cargo test --all-targets range` | range parser/semantic regression 검증 |
| C2 | done | `scripts/check.sh` | full native smoke 유지 |
