# Open Questions: B3 Self-Hosting C Backend

Status: active; P177a decisions frozen unless later evidence reopens them

## Should The Mallang Backend Port The Entire Rust Backend At Once?

No. It grows by typed-IR capability slice. Every slice must have a focused
fixture, byte-level Stage0 differential and native safety evidence before the
next ownership surface is added.

## What Output Is Deterministic?

Generated C is the fixed output contract. Native executable bytes are excluded
because linker and platform metadata are outside the compiler contract.

## May B3 Add Public Syntax Or Standard APIs?

Only when a representative compiler source is blocked and the change is a
separately reviewed, backward-compatible 1.x addition. P177a adds none.

## Which Gate Runs During Normal Editing?

Use the artifact-reuse backend gate after focused tests. Rebuild Stage1 when
compiler sources or Rust Stage0 changed, run compiler-source IR differential
for ownership/IR changes, and reserve the canonical full gate for a completed
logical change and publication.

## How Much Host Driver Is Allowed?

B3 may retain deterministic source discovery, process invocation and `clang`
execution in the host. C generation semantics and generated source ownership
must live in Mallang. P177d records the exact remaining host boundary for B4.
