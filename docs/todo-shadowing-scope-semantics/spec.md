# Spec: shadowing-scope-semantics

## Goal

- Mallang v0 binding shadowing 규칙을 lexical block 기준으로 고정한다.
- Same-block redeclaration은 계속 reject하고, nested block shadowing은 허용한다.
- Shadowed inner binding의 move 상태가 outer binding으로 merge되지 않게 한다.

## Scope

- Semantic checker local binding에 lexical scope identity를 추적한다.
- `if`, `for`, `range` body의 nested block shadowing을 허용한다.
- Branch/loop move merge는 같은 binding identity일 때만 outer state에 반영한다.
- `for`/`range` body native C lowering은 body-local shadowing이 header/range
  binding이나 post clause와 같은 C block에서 충돌하지 않도록 user body를 nested
  C block으로 감싼다.

## Non-goals

- First-class references or statement-spanning borrow lifetimes.
- General hygienic renaming for every local binding.
- Package/module scope shadowing policy.

## Acceptance

| ID | Status | Evidence |
| --- | --- | --- |
| C1 | done | `cargo test shadow` |
| C2 | done | `scripts/check.sh` native smoke for `examples/shadowing.mlg` |
| C3 | done | `SPEC.md` binding rules updated |
