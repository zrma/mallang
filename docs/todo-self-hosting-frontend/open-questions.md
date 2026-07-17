# Open Questions: B1 Self-Hosting Frontend

Status: initial B1 decisions resolved; later blockers must be recorded here

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
