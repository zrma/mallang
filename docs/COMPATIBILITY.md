# Mallang Versioning and Compatibility Policy

Status: stable from the v1.0.0 release

This policy defines how compiler releases, the language contract, the standard
library, the CLI, and supported native targets evolve. The rule-indexed language
surface is `docs/V1_LANGUAGE_CONTRACT.md`.

## Version model

Mallang stable releases use semantic versions in the form `major.minor.patch`.
A release candidate appends a SemVer prerelease suffix, such as
`1.0.0-rc.1`. One release version identifies the `mlg` compiler, the language
contract it implements, the compiler-owned standard packages, the installer,
and the native archives. Standard packages are not independently versioned.

- Through v0.9, the compiler package version and candidate language-specification
  line advance together.
- v0.9.0 freezes and validates the candidate contract but is still a pre-1.0
  release.
- v1.0.0-rc.1 rehearses the frozen contract, installation, upgrade and rollback
  path but does not begin the stable 1.x compatibility guarantee.
- v1.0.0 is the first stable implementation of the Mallang v1 contract.
- Every v1.x compiler implements the same Mallang v1 language version. A v1
  source file does not select a compiler minor version.

Mallang v1 has no edition, manifest language-version field, source pragma, or
per-project compatibility switch. `mallang.toml` therefore selects project and
dependency structure only. An edition mechanism requires a concrete future
compatibility need and a separate major-version design; it is not reserved as a
silent escape hatch for a breaking 1.x change.

## Compatibility unit

A **valid v1 program** is source that conforms to the v1 contract and is accepted
by the v1.0.0 compiler. For the same program inputs and documented platform
conditions, **observable semantics** include:

- evaluation order, ownership transfer, mutation, normal-flow cleanup, and
  fatal no-unwind boundaries;
- native stdout, stderr, exit status, file effects, and standard-library return
  values;
- accepted project/package/dependency structure;
- stable CLI commands, options, diagnostic schema fields, and exit classes.

Programs that rely on rejected source, compiler crashes, undefined host behavior,
undocumented generated symbols, native layouts, or other explicitly internal
behavior are not valid v1 programs for compatibility purposes.

## 1.x guarantees

Within the v1 major line:

- A later v1.x compiler MUST continue to accept every valid v1 program.
- It MUST NOT silently change that program's observable semantics.
- It MUST preserve the exact public standard-library signatures and their
  ownership, failure, and UTF-8 behavior. New packages or functions MAY be added
  only under the already reserved `std/...` namespace.
- It MUST preserve stable user CLI commands, option meanings, machine diagnostic
  schema, release archive shape, and installer verification behavior.
- It MUST preserve macOS arm64 and Linux x86_64 support. Adding a target is
  compatible; removing or weakening a supported target requires a major release.
- New syntax MUST NOT turn a previously valid identifier into a reserved word.
  Additive syntax in 1.x must be unambiguous or contextual for existing source.

Exact human diagnostic wording, successful `lex`/`parse`/`ir` inspection output,
generated C spelling, native ABI/layout, compiler performance, native executable
bytes, and archive bytes produced by different compiler versions are not 1.x
compatibility guarantees. Their documented schema, safety boundary, or
same-compiler reproducibility contract still applies.

## Release classes

| Release | Allowed change | Compatibility requirement |
| --- | --- | --- |
| Patch `1.x.z` | Correct implementation bugs, soundness defects, diagnostics, documentation, packaging, and performance. | MUST preserve valid source and observable semantics except for the soundness exception below. |
| Minor `1.y.0` | Add backward-compatible syntax, APIs, diagnostics, tooling, or supported targets. | MUST accept all earlier v1 source and preserve its semantics. |
| Major `2.0.0` | Remove or change syntax, types, ownership, standard APIs, CLI contracts, target support, or observable semantics. | MUST provide release notes and a concrete migration path for every intentional break. |

A change is breaking when it rejects a valid v1 program, changes its type or
ownership meaning, changes its observable result, removes a stable API/command,
or removes supported target behavior. Relabeling such a change as a bug fix does
not make it compatible.

Correcting an implementation that contradicts an existing normative rule is not
a change to the language contract, even when the incorrect compiler produced a
different result. A user-visible correction still belongs in release notes and
MUST include a regression that names the restored rule.

## Deprecation

Deprecation does not invalidate source in the current major line.

1. A deprecated source or API surface remains accepted throughout v1.x.
2. Documentation and release notes MUST identify its replacement and planned
   major-version removal.
3. When compiler detection is practical, a diagnostic SHOULD precede removal by
   at least one minor release. Until a warning channel exists for that surface,
   documentation and release notes are the minimum notice.
4. Removal or a source-breaking semantic change occurs only in the next major
   version.

Rule IDs are never recycled. A deprecated rule remains in the contract with its
status and replacement; a successor receives a new ID.

## Soundness and security exception

Mallang may reject previously accepted source in a v1 patch or minor release only
when accepting it violates the stated memory-safety contract, permits unchecked
native behavior, or creates a security vulnerability that cannot be fixed while
preserving the program.

Such a change MUST:

- be the narrowest rejection that closes the defect;
- identify affected rule IDs and source patterns;
- include a regression test and migration or safe replacement;
- be called out as a compatibility exception in release notes.

Ordinary compiler bugs, implementation convenience, refactoring, or performance
work do not qualify for this exception.

## Pre-1.0 and freeze policy

The v0.x series does not promise cross-minor source compatibility. Removed or
normalized 0.x forms are documented in the v1 migration guide rather than kept as
legacy aliases. From the v0.9 feature freeze through v1.0.0, source-visible changes
are limited to resolving a soundness defect or an explicit contradiction between
the candidate contract and the released implementation. Bug, diagnostic,
documentation, conformance, migration, and release-workflow fixes remain allowed.

## Change classification

Every source-visible or distribution-visible change after the freeze records:

1. affected `V1-*` rule IDs;
2. a valid before/after source or workflow example;
3. source-acceptance and observable-semantic impact;
4. release class and, if needed, deprecation or soundness-exception treatment;
5. conformance and migration evidence.

If compatibility impact is unclear, the change does not enter a release until a
decision gate resolves it.
