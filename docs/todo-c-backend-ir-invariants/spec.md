# Spec: c-backend-ir-invariants

## 목표

- C backend가 malformed typed IR을 조용히 C로 emit하지 않고 invariant error로 실패하는지 regression으로 고정한다.
- 정상 parser/semantic/lowerer 경로가 만들 수 없는 IR도 backend public boundary에서는 방어적으로 다룬다.
- full C backend gap review 전에 compile/IR boundary의 최소 안전망을 넓힌다.

## 결정

- 수동 `IrProgram`을 구성해 backend public API `generate_c_from_ir`의 error surface를 직접 검증한다.
- 정상 Mallang source에서 발생하는 user diagnostic이 아니라 internal invariant failure로 분류한다.
- 현재 slice는 representative invariant만 고정하고, 전체 malformed IR fuzzer나 validator는 만들지 않는다.

## 범위

- `print` call arity invariant.
- `range` source type invariant.
- ADT match arm/type invariant.
- Borrow argument lvalue invariant.
- ROADMAP/HANDOFF 갱신.

## 제외

- 별도 IR validator pass.
- 모든 `IR invariant violation` branch의 exhaustive test.
- Panic-free compiler-wide error taxonomy 정비.

## C-체크리스트

| ID | 상태 | Verify command | 작업 항목 |
| --- | --- | --- | --- |
| C1 | done | `cargo test --all-targets backend::c::tests::rejects_invalid_ir` | C backend malformed IR regression |
| C2 | done | `cargo clippy --all-targets -- -D warnings` | Rust lint 유지 |
