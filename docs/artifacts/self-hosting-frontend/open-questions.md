# Open Questions: B1 Self-Hosting Frontend

Status: closed with B1; later frontend changes require a new decision record

## Q1. How does Mallang scan immutable source text?

Use `strings.byteAt` for safe byte inspection and `strings.slice` for owned
lexemes. Both report recoverable `InvalidInput`; no borrowed substring or
pointer-like string view is introduced.

## Q2. What coordinate system do source spans use?

Zero-based UTF-8 byte offsets with an exclusive end, matching Stage0 and the
existing `byteLen` and `find` APIs. Line and column rendering is derived from the
source and is not stored as frontend identity.

## Q3. What is compared between frontends?

A stable normalized representation owned by the differential harness, not Rust
`Debug` output or C struct layout. Token kind, lexeme payload, byte span, AST
shape and normalized frontend diagnostic fields are compared exactly.

## Q4. Does the Mallang AST include semantic information?

No. B1 owns syntax only. Name resolution, inferred types, ownership state and
specialization belong to B2 and later stages.

## Q5. May B1 initially support a grammar subset?

Implementation may land in tested slices, but B1 is not complete until the
entire frozen v1 grammar and frontend rejection corpus agree with Stage0.

## Q6. Why is the bootstrap token kind a string tag rather than an enum?

The current ownership model treats user-defined enums as move-only values, so a
read-only token normalizer cannot inspect an enum field through `con` without
either copying or moving it. P175b uses a private stable string tag such as
`Keyword.Func` inside the bootstrap compiler instead of expanding the public
language solely for compiler convenience. The Rust oracle maps its enum to the
same harness-owned tag. A later typed compiler representation may change this
private data structure without changing Mallang syntax or the differential
contract.

## Q7. Which sources form the final B1 corpus?

All `.mlg` files under `bootstrap/compiler/src`, `bootstrap/compiler/tests`,
`examples` and `tests/fixtures`, sorted by repository-relative path. This keeps
valid programs, syntax and lexical rejection cases, semantic-only rejection
cases and the hardening crash corpus under one deterministic frontend contract.
The gate rejects an unexpectedly small corpus instead of silently accepting an
empty or truncated discovery result.
