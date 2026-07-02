# Spec: backend-c-test-module-split

## 목표

- C backend unit tests를 `src/backend/c.rs`에서 분리해 production orchestration file을 얇게 유지한다.
- Backend implementation split 이후 test layout도 module boundary에 맞춘다.

## 범위

- `src/backend/c.rs`
  - `#[cfg(test)] mod tests;` declaration만 유지
  - production C output orchestration과 `CGenerator` boundary 유지
- `src/backend/c/tests.rs`
  - 기존 C backend unit tests 이동
  - 기존 `backend::c::tests::*` test path 유지
- 문서/roadmap/handoff 갱신

## 제외

- C backend behavior 변경
- C output format 변경
- test case 추가/삭제
- test helper abstraction 추가

## C-체크리스트

| ID | 상태 | Verify command | 작업 항목 |
| --- | --- | --- | --- |
| C1 | done | `cargo test backend::c::tests --all-targets` | C backend test path 유지 검증 |
| C2 | done | `scripts/check.sh` | full C backend behavior smoke |
