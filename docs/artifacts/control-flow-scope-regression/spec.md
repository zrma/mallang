# Spec: control-flow-scope-regression

Status: complete; historical milestone record

## Goal

- Nested shadowing scope가 IR/backend lowering 전체에서 유지되는지 control-flow별
  회귀 테스트로 고정한다.

## Scope

- `match` expression payload binding이 outer binding과 같은 이름을 써도 arm-local
  scope로 처리되는지 native smoke로 검증한다.
- Statement `match` arm에서 payload binding을 move해도 outer binding이 move되지 않는지
  semantic test로 검증한다.
- Condition-only `for` body가 condition binding과 같은 이름을 shadow해도 loop condition과
  body-local binding이 분리되는지 semantic/native smoke로 검증한다.

## Non-goals

- New syntax or user-visible scope feature changes.
- Full hygienic renaming in IR.

## Acceptance

| ID | Status | Evidence |
| --- | --- | --- |
| C1 | done | `cargo test shadow` includes match and condition-loop shadowing cases |
| C2 | done | `scripts/check.sh` runs expanded `examples/shadowing.mlg` native smoke |
| C3 | done | `SPEC.md` mentions arm-local shadowing scope |
