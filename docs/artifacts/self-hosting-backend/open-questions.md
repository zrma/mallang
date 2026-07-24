# Open Questions: B3 Self-Hosting C Backend

Status: closed with B3; decisions frozen unless later evidence reopens them

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

B3 retains deterministic source discovery, project-graph argument assembly and
`clang` invocation in the host harness. Compiler semantics and generated source
ownership live in Mallang. B4 treats the ordered source set and graph as
declared inputs while proving Stage1-to-Stage2 identity; B5 owns migration of
the public project-discovery and native-build workflow to the Mallang compiler.
