# Spec: by-reference-range-decision

Status: complete; historical milestone record

## 목표

- v0에서 by-reference range binding syntax를 열지 않는다.
- Range element borrowing은 기존 indexed borrow argument surface로 유지한다.
- Future borrowed iteration은 statement-spanning borrow lifetime 설계 뒤에 연다.

## 결정

- `for i, con value := range values`는 v0 syntax가 아니다.
- `for i, mut value := range values`도 v0 syntax가 아니다.
- Element read borrow는 `con values[i]`, mutable borrow는 `mut values[i]`,
  mutation은 `values[i] = expr`를 사용한다.
- Future by-reference range design은 range body 전체에 걸친 borrow lifetime과
  overlap rules를 먼저 정의해야 한다.

## 범위

- Parser regression: `con` range value binding syntax reject.
- SPEC/roadmap/handoff 갱신.

## 제외

- By-reference range implementation.
- Statement-spanning borrow lifetime implementation.
- IR/backend range representation 변경.

## C-체크리스트

| ID | 상태 | Verify command | 작업 항목 |
| --- | --- | --- | --- |
| C1 | done | `cargo test --all-targets range` | range parser regression 검증 |
| C2 | done | `scripts/check.sh` | full native smoke 유지 |
