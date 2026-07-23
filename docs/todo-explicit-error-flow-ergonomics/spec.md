# Spec: explicit-error-flow-ergonomics

Status: deferred; explicit match-return design recorded for a compatible 1.x milestone

## Problem

Mallang v1 keeps recoverable failure explicit through `Result[T, E]` and
exhaustive `match`. That contract is clear, but compiler orchestration such as
`bootstrap/compiler/src/main.mlg` accumulates one indentation level for every
successful `Result` step. Helper extraction reduces individual function size
but does not remove the repeated unwrap-and-return shape.

`?` is not the chosen direction. It would hide both the exact propagation point
and error mapping, and `V1-STD-008` explicitly excludes it from v1.

## Recommended Surface

Keep `match` and `return` visible:

```mlg
program := match parseSourcePaths(con args, argument) {
    case Ok(program) {
        program
    }
    case Err(errors) {
        return Err(normalizeErrors(con errors))
    }
}
```

The success arm produces the bound value. The error arm explicitly maps and
returns the failure, so the remainder of the function stays at the surrounding
indentation level.

## Semantic Model

- An expression `match` arm is either value-producing or unconditionally
  function-returning.
- A returning arm has an internal diverging/bottom result used only for type
  convergence. Mallang does not expose a source-level `Never` type.
- At least one reachable arm must produce a non-unit value. All value-producing
  arms keep the existing same-type rule.
- `return` keeps its existing function return-type check and ownership transfer.
- Locals owned by a returning arm are dropped before the function return;
  surviving value arms keep existing branch move-merge behavior.
- Exhaustiveness, pattern bindings, evaluation order, and scrutinee ownership
  do not change.
- Stage0 and self-hosted parser, semantic, typed IR, cleanup, and backend paths
  must accept and lower the same source.

## Initial Boundary

- Allow ordinary statements followed by one final expression in a
  value-producing match arm.
- Allow ordinary statements ending in an unconditional `return` in a diverging
  arm.
- Do not initially generalize `break`, `continue`, panic, process exit, arbitrary
  block expressions, or `if` expression divergence.
- Do not add `?`, exceptions, implicit return, implicit error conversion,
  nullable chaining, or stack unwinding.
- Do not add a public `Never` spelling until a separate use case requires it.

## Immediate Source-Level Practice

Before the syntax lands, deep orchestration should be split into helpers that
return one normalized `Result`. The outer command boundary then uses one
statement-form exhaustive `match`. This reduces function depth without changing
the language and provides representative acceptance cases for the later syntax.

## Compatibility

This accepts previously rejected source without changing existing valid v1
program behavior or reserving a new identifier. It is therefore eligible for a
minor 1.x release, but only after the rule-index impact and Stage0/self cleanup
parity are reviewed. Existing `V1-CTL-006` wording must be refined to quantify
value-producing arms rather than silently contradicted.

## Acceptance

- Parser positive/rejection tests distinguish value and returning arms.
- Semantic tests cover return-type mismatch, no value-producing arm, mixed
  value types, exhaustiveness, moves, and borrowed payloads.
- IR/backend tests cover arm locals, outer cleanup roots, returned owned errors,
  and value-arm temporaries.
- Native tests cover `Ok` continuation and mapped `Err` early return.
- The self-hosted compiler uses the surface in at least one orchestration path
  and remains fixed-point clean.
- `?` remains rejected by lexer/parser and absent from the language contract.
