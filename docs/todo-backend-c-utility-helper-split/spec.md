# Spec: backend-c-utility-helper-split

## 목표

- C backend의 shared formatting, temp-name, checked-int helper, parameter-env helper를 orchestration module에서 분리한다.
- `src/backend/c.rs`를 C output orchestration과 `CGenerator` boundary 중심으로 유지한다.

## 범위

- `src/backend/c/utils.rs`
  - `finish_with_prelude`
  - `push_indented_lines`
  - expression/statement temp-name helpers
  - checked integer builtin selection
  - `param_env`
  - blank identifier helper
- `src/backend/c.rs`
  - module registration
  - public `generate_c` / `generate_c_from_ir`
  - `CGenerator` orchestration
- `src/backend/c/expressions.rs`, `src/backend/c/statements.rs`, `src/backend/c/types.rs`
  - shared helper imports를 `utils` module로 전환

## 제외

- C output format 변경
- type/statement/expression emitter behavior 변경
- backend trait abstraction
- C backend tests 재배치

## C-체크리스트

| ID | 상태 | Verify command | 작업 항목 |
| --- | --- | --- | --- |
| C1 | done | `cargo check --workspace` | utility helper module compile 검증 |
| C2 | done | `scripts/check.sh` | full C backend behavior smoke |
