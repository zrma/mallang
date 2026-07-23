# Open Questions: B5 Default Self-Hosted Compiler

Status: active; P179a decisions frozen unless implementation evidence reopens them

## Does The Public Command Change?

No. Users continue to invoke `mlg`. Implementation selection is an internal
driver and packaging concern, not a reason to add another normal compiler name.

## What Is The Rust Stage0 Artifact?

The tracked Rust source and locked Cargo dependency graph are the authoritative
seed. A prebuilt Stage0 executable may be a release convenience but is never
the only recovery input or the source of truth.

## May The Driver Silently Fall Back To Stage0?

No. Silent fallback can hide self-hosted regressions and invalidate dogfood
evidence. Stage0 use must be explicitly selected and visible in diagnostics or
machine-readable provenance.

## Must The Rust Host Driver Disappear In B5?

Not necessarily. A narrow launcher may remain for artifact selection and native
process invocation if all compiler semantics and public command behavior are
owned by Mallang and the boundary is documented. Retaining a large second
compiler implementation behind the default path is not acceptable.

## When May The Default Switch Happen?

Only after self-hosted project discovery, diagnostics, formatting, testing,
native workflow and release packaging pass complete Stage0/default parity on
every supported platform. B4 fixed-point evidence alone is insufficient.
