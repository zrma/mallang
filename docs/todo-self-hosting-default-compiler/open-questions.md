# Open Questions: B5 Default Self-Hosted Compiler

Status: resolved for B5

## Does The Public Command Change?

No. Users continue to invoke `mlg`. Implementation selection is an internal
driver and packaging concern, not a reason to add another normal compiler name.
The internal self-hosted engine is `mlgc`; users do not need a second normal
compiler command.

## How Is The Implementation Selected During B5?

`--compiler stage0|self` is the explicit selector. No flag selects the
self-hosted core. `--self-compiler <path>` is a development override and
requires explicit `--compiler self`; ordinary layouts resolve sibling `mlgc`.
No environment-variable search or silent fallback is allowed. Explicit
`--compiler stage0` remains the offline recovery and differential-oracle path.

`mlg --version` retains its stable one-line output. `--version --verbose` adds
driver, selected implementation and core protocol provenance.

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

The local switch happened only after self-hosted project discovery,
diagnostics, formatting, testing, native workflow and dual-binary release
packaging passed Stage0/default parity. P179d then passed the same default,
fixed-point, archive and clean-install evidence on every supported platform.
B4 fixed-point evidence alone was insufficient; B5 publication remains gated by
the P179e signed-release and installed-artifact verification.
