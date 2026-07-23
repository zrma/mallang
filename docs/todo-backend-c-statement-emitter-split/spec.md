# Spec: backend-c-statement-emitter-split

Status: complete; historical milestone record

## 목표

- C backend의 statement/loop/match statement/print emission을 expression emission과 분리한다.
- 후속 expression emitter 분리 전에 statement-level responsibility boundary를 고정한다.

## 범위

- `src/backend/c/statements.rs`
  - `IrStmt` lowering
  - `if`, `for`, `range`, `match` statement lowering
  - statement-form `print` emission
  - assignment target lowering and index assignment statement lowering
  - cleanup statement emission helper
- `src/backend/c.rs`
  - C output orchestration과 expression emission 유지
  - existing `generate_c` / `generate_c_from_ir` public API 유지
- 문서/roadmap/handoff 갱신

## 제외

- expression emitter module split
- type emitter/name helper 변경
- backend trait abstraction
- C output format 변경

## C-체크리스트

| ID | 상태 | Verify command | 작업 항목 |
| --- | --- | --- | --- |
| C1 | done | `cargo check --workspace` | statement emitter module compile 검증 |
| C2 | done | `scripts/check.sh` | full C backend behavior smoke |
