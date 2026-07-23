# Spec: backend-c-type-emitter-split

Status: complete; historical milestone record

## 목표

- C backend의 type collection, C type layout emission, cleanup helper emission을 function/statement/expression emission orchestration에서 분리한다.
- 후속 statement/expression emitter 분리 전에 type-level responsibility boundary를 고정한다.

## 범위

- `src/backend/c/types.rs`
  - typed IR에서 C에 필요한 defined type 목록 수집
  - `Option`, `Result`, struct, fixed-size array, owned slice `typedef` emission
  - cleanup-capable type의 `mlg_drop_*` helper emission
- `src/backend/c.rs`
  - `generate` orchestration에서 type emitter module 호출
  - function/statement/expression emission 책임 유지
  - existing `generate_c` / `generate_c_from_ir` public API 유지
- 문서/roadmap/handoff 갱신

## 제외

- statement emitter module split
- expression emitter module split
- backend trait abstraction
- C output format 변경

## C-체크리스트

| ID | 상태 | Verify command | 작업 항목 |
| --- | --- | --- | --- |
| C1 | done | `cargo check --workspace` | type emitter module compile 검증 |
| C2 | done | `scripts/check.sh` | full C backend behavior smoke |
