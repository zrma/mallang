# Spec: Self-Hosting Bootstrap

Status: complete; B0 contract and feasibility accepted on 2026-07-17

## Goal

Define a reproducible, auditable path from the Rust compiler to a Mallang
compiler and prove that the current Stage0 can build a compiler-shaped Mallang
project without changing the stable v1 source contract.

## B0 Scope

- define Stage0, Stage1 and Stage2 without claiming that the probe is a compiler
- define fixed-point evidence and explicitly exclude native binary byte identity
- preserve Rust Stage0 as the trusted seed and differential oracle
- allow a bounded host driver while assigning compiler semantics to Mallang code
- inventory language and standard-library gaps without pre-approving new surface
- add a tracked Mallang probe built, tested and executed by Stage0
- verify same-input generated C byte identity for the probe

## Acceptance

- [x] `docs/SELF_HOSTING.md` owns the durable B0-B5 contract
- [x] `open-questions.md` has no unresolved B0 decision
- [x] Stage0 is built from the current Rust source with the locked dependency set
- [x] Stage0 format, check and project tests pass for `bootstrap/probe`
- [x] Stage0 builds a native probe whose exact output is checked
- [x] two independent probe builds produce byte-identical generated C
- [x] canonical repository and publication gates include the B0 artifacts

## Excluded

- lexer/parser implementation, which starts in B1
- semantic, IR or C backend porting
- Stage1 or Stage2 compiler claims
- default compiler transition
- public syntax, standard-library or version changes without blocker evidence
- deleting or freezing the Rust compiler implementation
