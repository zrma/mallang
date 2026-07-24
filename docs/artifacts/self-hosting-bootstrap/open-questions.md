# Open Questions: Self-Hosting Bootstrap

Status: B0 decisions resolved from the approved staged self-hosting direction

## Q1. What is the trusted seed?

The Rust implementation built from the repository's locked dependency set is
Stage0. Signed release artifacts provide stable bootstrap checkpoints, while the
current source build remains the development oracle.

## Q2. What proves self-hosting?

Stage0 must build Stage1, and Stage1 must build Stage2 from the identical
Mallang compiler source set. Stage1 and Stage2 compiler-generated C must be
byte-identical and their conformance behavior must agree. Native executable byte
identity is explicitly excluded.

## Q3. Can a host driver remain during the port?

Yes, temporarily. It may discover an ordered source set, invoke the current
compiler stage and run `clang`. It may not own lexical, parsing, semantic,
ownership, IR, specialization or C-generation decisions.

## Q4. When is the Rust compiler removed?

It is not removed by this program. B5 changes the default release path while
retaining Rust Stage0 as a documented seed, audit target and differential oracle.

## Q5. May self-hosting add language features?

Only after representative compiler code produces a concrete blocker. Every
addition must be backward-compatible in 1.x, narrowly specified and covered by
the normal conformance and release gates.

## Q6. When does the Mallang compiler become the default?

Only after B4 fixed-point acceptance passes on every supported platform and B5
rehearses clean bootstrap, upgrade, rollback and release artifact production.
