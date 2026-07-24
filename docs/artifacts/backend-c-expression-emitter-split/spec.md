# Spec: backend-c-expression-emitter-split

Status: complete; historical milestone record

## 목표

- C backend의 expression/literal/call/borrow-lvalue/match-expression emission을 statement/orchestration module에서 분리한다.
- 후속 helper/temp-name utility split 전에 expression-level responsibility boundary를 고정한다.

## 범위

- `src/backend/c/expressions.rs`
  - `IrExpr` lowering
  - expression-form `if`/`match` lowering
  - literal, field/index/len, call, borrow-lvalue lowering
  - array/slice literal, append, slice field take expression lowering
  - checked integer unary/binary lowering
- `src/backend/c.rs`
  - C output orchestration과 shared helper/temp-name utilities 유지
  - existing `generate_c` / `generate_c_from_ir` public API 유지
- 문서/roadmap/handoff 갱신

## 제외

- helper/temp-name utility module split
- type/statement emitter 변경
- backend trait abstraction
- C output format 변경

## C-체크리스트

| ID | 상태 | Verify command | 작업 항목 |
| --- | --- | --- | --- |
| C1 | done | `cargo check --workspace` | expression emitter module compile 검증 |
| C2 | done | `scripts/check.sh` | full C backend behavior smoke |
