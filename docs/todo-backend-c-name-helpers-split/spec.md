# Spec: backend-c-name-helpers-split

## 목표

- C backend 내부에서 C identifier/type/operator naming helper를 emission orchestration에서 분리한다.
- 이후 type/statement/expression emitter 분리의 선행 경계를 만든다.

## 범위

- `src/backend/c/names.rs`
  - `TypeCName`, `IrAdtConstructorCName`, `COperator`
  - `c_ident`, `c_field`, `c_param_decl`, `c_assignment_target`, `c_arg_code`
  - `c_condition`, `c_binary_expr`, `drop_fn_name`, `empty_slice_value_code`, `c_string`
- `src/backend/c.rs`
  - helper를 import해서 기존 codegen behavior와 public API 유지
- 문서/roadmap/handoff 갱신

## 제외

- type/statement/expression emitter module split
- backend trait abstraction
- C output format 변경

## C-체크리스트

| ID | 상태 | Verify command | 작업 항목 |
| --- | --- | --- | --- |
| C1 | done | `cargo check --workspace` | helper module compile 검증 |
| C2 | done | `scripts/check.sh` | full C backend behavior smoke |
