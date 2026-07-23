# Spec: B5 Default Self-Hosted Compiler

Status: active; P179a and P179b1 complete, P179b2 pending

## Objective

B5 makes the Mallang implementation the default compiler behind the existing
`mlg` command. The tracked Rust implementation remains a reproducible Stage0
bootstrap seed, differential oracle and explicit rollback path. The transition
must not weaken the frozen v1 language, ownership, diagnostic, packaging or
supported-platform contracts.

## Default And Seed Contract

- `mlg` remains the public command and package name.
- The default frontend, semantic/ownership checker, typed IR and C generator
  come from the fixed-point Mallang compiler.
- The Rust source under the tracked Cargo workspace is the authoritative
  Stage0 seed; no opaque prebuilt binary is trusted or required.
- `cargo build --locked --bin mlg` remains the reproducible seed build path.
- Until B5 release acceptance closes, an explicit Stage0 selector remains
  available for differential diagnosis and rollback.
- The selector is not silent fallback. A self-hosted compiler failure must be
  reported unless the operator explicitly requests Stage0.
- The transition names are `mlg` for the public driver, `mlgc` for the internal
  self-hosted engine and `--compiler <stage0|self>` for explicit selection.
- `--self-compiler <path>` is an explicit development/diagnostic override. In
  ordinary installed layouts, `mlg` resolves sibling `mlgc` without an ambient
  environment-variable search.

## Work Breakdown

### P179a: Transition Contract And Harness

- freeze public command, internal self-hosted executable and Stage0 selector
  names without changing normal `mlg` UX
- define artifact provenance and version output for the driver, self-hosted
  compiler and Rust seed
- add parity harnesses that run the public command through both implementations
- define a non-recursive bootstrap build graph and clean-checkout recovery path

P179a is complete. `scripts/build-self-hosted-compiler.sh` builds tracked Rust
Stage0, strict-C11 Stage1 and fixed Stage2, then installs the Stage2 engine as
`mlgc`. The public driver defaults to Stage0 for now, accepts explicit
`--compiler stage0|self`, reports driver/compiler/core provenance through
`--version --verbose` and never silently falls back. The self path currently
owns standalone `check`, `build` and `run`; commands reserved for later
P179b-P179c slices fail with an explicit transition diagnostic. The isolated
`scripts/check-self-hosting-default-compiler.sh` gate compares generated C,
native output, status, stderr and human/JSON rejection diagnostics through the
public driver and runs in the macOS arm64/Linux x86_64 CI matrix.

### P179b: Public Project And Diagnostic Surface

- move project discovery, manifest loading and dependency graph assembly behind
  the Mallang compiler boundary
- preserve source ordering, source paths, human/JSON diagnostics and exit codes
- expose public `check`, `ir` and `build` behavior through the self-hosted core
- retain explicit Stage0 differential coverage for every command

P179b1 is complete. The Rust host driver decodes the Mallang compiler's bounded
`PERR`, `KERR`, `LERR`, `SERR` and `IERR` records into the existing public
human/JSON diagnostic schema, validates source IDs, byte spans and UTF-8, and
routes standalone `check`, `build` and `run` failures through that boundary.
Malformed internal output is a backend protocol error and never triggers a
Stage0 fallback. The transition gate proves successful standalone `check`,
semantic rejection and multi-error parser recovery parity in both diagnostic
formats. P179b2 owns project discovery, manifest/dependency graph assembly,
project `check`/`build` and public IR output.

### P179c: Tooling And Native Workflow

- move formatter, test selection/runner generation and native process workflow
  behind the Mallang implementation
- preserve `fmt`, `test`, `run`, install and generated-C contracts
- retain strict C11, allocation accounting, sanitizer and crash-corpus evidence

### P179d: Default Switch And Packaging

- make release and install artifacts invoke the self-hosted compiler by default
- package the minimum reproducible bootstrap metadata and document Stage0
  recovery from a clean checkout
- validate macOS arm64 and Linux x86_64 archives, checksums and installed UX
- keep explicit rollback available without network access

### P179e: B5 Closure

- run complete Stage0/default differential, fixed-point and release gates
- pass publication boundary and private-inventory gates
- publish the compatible 1.x release selected by release evidence
- verify signed tag, GitHub Release assets, checksums and clean install
- document the long-term seed refresh and bootstrap audit policy

## Acceptance

- [x] public `mlg` and internal compiler/seed naming contract
- [x] deterministic clean-checkout Stage0 -> Stage1 -> Stage2 build graph
- [x] explicit non-silent Stage0 diagnostic and rollback selector
- [ ] self-hosted public project discovery, diagnostics, check, IR and build
- [ ] self-hosted format, test, run and native process workflow
- [ ] complete Stage0/default command and conformance parity
- [ ] default release artifacts use the Mallang compiler core
- [ ] macOS arm64 and Linux x86_64 packaging and clean-install acceptance
- [ ] B5 publication, signed tag and GitHub Release acceptance

## Excluded

- deleting the Rust Stage0 source or differential oracle
- changing the public `mlg` command solely for implementation provenance
- silently falling back to Rust after a self-hosted compiler failure
- requiring network access or an untracked binary to recover the compiler
- incompatible 2.0 naming or syntax changes
