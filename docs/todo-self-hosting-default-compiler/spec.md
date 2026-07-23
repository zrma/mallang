# Spec: B5 Default Self-Hosted Compiler

Status: active; P179a planning complete, implementation pending

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

## Work Breakdown

### P179a: Transition Contract And Harness

- freeze public command, internal self-hosted executable and Stage0 selector
  names without changing normal `mlg` UX
- define artifact provenance and version output for the driver, self-hosted
  compiler and Rust seed
- add parity harnesses that run the public command through both implementations
- define a non-recursive bootstrap build graph and clean-checkout recovery path

### P179b: Public Project And Diagnostic Surface

- move project discovery, manifest loading and dependency graph assembly behind
  the Mallang compiler boundary
- preserve source ordering, source paths, human/JSON diagnostics and exit codes
- expose public `check`, `ir` and `build` behavior through the self-hosted core
- retain explicit Stage0 differential coverage for every command

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

- [ ] public `mlg` and internal compiler/seed naming contract
- [ ] deterministic clean-checkout Stage0 -> Stage1 -> Stage2 build graph
- [ ] explicit non-silent Stage0 diagnostic and rollback selector
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
