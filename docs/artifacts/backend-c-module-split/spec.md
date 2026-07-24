# Spec: backend-c-module-split

Status: complete; historical milestone record

## 목표

- Native backend 분리의 첫 구조 작업으로 backend public API와 C backend
  implementation을 파일 경계로 분리한다.
- 기존 CLI와 public library API는 유지한다.

## 범위

- `src/backend/mod.rs`:
  - backend module의 public API boundary로 사용한다.
  - `generate_c`, `generate_c_from_ir`, `CompileError`를 re-export한다.
- `src/backend/c.rs`:
  - 기존 C backend implementation과 unit tests를 보존한다.
  - 기존 `generate_c` / `generate_c_from_ir` public 함수 shape를 유지한다.
- 문서:
  - README layout과 agent roadmap/handoff에 module split을 반영한다.

## 제외

- C backend 내부 type/statement/expression emitter 세분화.
- LLVM/Cranelift backend 추가.
- Backend trait abstraction.
- CLI command shape 변경.

## C-체크리스트

| ID | 상태 | Verify command | 작업 항목 |
| --- | --- | --- | --- |
| C1 | done | `cargo check --workspace` | module path split compile 검증 |
| C2 | done | `scripts/check.sh` | public API와 native smoke 회귀 검증 |
