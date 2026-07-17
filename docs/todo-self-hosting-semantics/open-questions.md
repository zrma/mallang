# Open Questions: B2 Self-Hosting Semantics And Typed IR

Status: active; decisions below are frozen unless later P176 evidence reopens them

## Q1. Does the checked model own a copy of the syntax arena?

No. B2 checks `con Program`, returns an owned `CheckedProgram`, and later typed
lowering receives both values explicitly. This avoids a second owned syntax tree
and does not introduce a reference field or first-class borrow.

## Q2. Is the canonical type string the semantic type system?

No. It is stable differential identity. The private resolved type also records
kind, Copy and cleanup classification, and later slices may add arena links or
nominal IDs without changing source syntax or normalized output.

## Q3. Why are private type kinds represented by strings?

The same reason as B1 token kinds: current user-defined enums are move-only and
cannot be inspected through `con` fields. A private string tag keeps repeated
read-only compiler traversal explicit without weakening the public ownership
model. It is not a source-language enum substitute.

## Q4. Which semantic diagnostic is compared?

Stage0 currently reports one semantic error after parser recovery. B2 therefore
compares the first semantic diagnostic exactly, including source ID, byte span
and message. Multi-error semantic recovery requires a separate future contract.

## Q5. Where does specialization run?

Specialization remains compiler-core work and must be implemented before B2
closes. P176a deliberately excludes generic declarations; P176d must match the
existing generic validation and specialization behavior before the full corpus
can pass.

## Q6. Can the temporary host driver build typed IR?

No. It may provide deterministic files, arguments and process execution only.
Name resolution, ownership state, specialization and every typed-IR decision
remain in tracked Mallang compiler source.
