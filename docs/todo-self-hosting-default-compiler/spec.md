# Spec: B5 Default Self-Hosted Compiler

Status: active; P179a-P179d complete; P179e v1.2.0 release acceptance active

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
`mlgc`. At the P179a boundary the public driver still defaulted to Stage0 while
accepting explicit `--compiler stage0|self`, reporting driver/compiler/core
provenance through `--version --verbose` and never silently falling back.
P179b-P179c subsequently moved the complete public compiler surface behind
`mlgc`. The isolated `scripts/check-self-hosting-default-compiler.sh` gate
compares generated C, native output, status, stderr and human/JSON rejection
diagnostics through the public driver and runs in the macOS arm64/Linux x86_64
CI matrix.

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
formats.

P179b2a is complete. The host exposes a read-only, root-first project-unit view
and passes that graph plus dependency-first source order to `mlgc`. Public
project `check`, `build` and `run` now execute Mallang-owned package, linker,
standard-library, specialization, semantic, IR and C-backend logic. The parity
gate covers a transitive local dependency graph, byte-identical generated C,
native output and dependency-source human/JSON diagnostics. A nested imported
generic type-argument regression found by this route is fixed and covered by a
linker/specializer integration test.

This is not yet the complete P179b boundary: Rust still finds and parses
manifests, resolves path dependencies, validates graph boundaries and enumerates
source files. P179b2b moves those decisions behind the Mallang boundary and
migrates public IR output without changing its user-visible contract.

P179b2b1 completes the first part of that move. The Mallang compiler now parses
the normative `[project]` and `[dependencies]` manifest surface, rejects unknown
or duplicate fields and emits a delimiter-safe, versioned manifest protocol.
Its focused gate compares all tracked manifests with the Rust `toml` oracle and
runs strict allocation and sanitizer checks.

P179b2b2a adds a Mallang graph planner over canonical host snapshots. It owns
project-name, dependency-path, key/name, collision, cycle, reachability and
dependency-postorder decisions and emits the root-first `PROJECT|1` protocol.
A local-dependency differential compares that plan with Rust through strict
allocation and sanitizer execution.

P179b2b2b connects both project protocols to the public self compiler path.
The Rust host resolves the initial manifest, brokers canonical filesystem paths
and enumerates source files, while `mlgc` parses every reachable manifest and
owns graph validation and dependency postorder. The host strictly decodes and
cross-checks the returned plan before materializing `Project`; malformed
protocol output remains a backend failure without fallback. The transition
gate proves `manifest`/`project-plan` routing and Stage0 parity for transitive
project `check`/`build`/`run`, invalid dependency paths and dependency cycles in
human and JSON formats.

P179b2b3 completes P179b by routing standalone and project `mlg ir` through the
self-hosted core. Stage0 and `mlgc` emit the same deterministic normalized
`IR|...` view instead of the former implementation-specific Rust debug output.
The host strictly validates record counts, source IDs, UTF-8 spans, encoded
payloads, node depths and child counts before publishing self-hosted output.
The transition gate compares all 48 typed-IR fixtures and a transitive project,
preserves human/JSON rejection parity and rejects malformed protocol output
without fallback.

### P179c: Tooling And Native Workflow

- move formatter, test selection/runner generation and native process workflow
  behind the Mallang implementation
- preserve `fmt`, `test`, `run`, install and generated-C contracts
- retain strict C11, allocation accounting, sanitizer and crash-corpus evidence

P179c1 moves canonical formatting behind the Mallang implementation. `mlgc`
parses and formats one source through a length-framed `FORMAT|1` response; the
Rust host validates the complete UTF-8 payload before comparing or writing it.
Project discovery follows the same Mallang-owned manifest/graph path as the
other public self commands, and all outputs are collected before any write.
The transition gate runs the standalone/project formatter smoke through both
implementations, compares the tracked example/compiler corpus byte-for-byte,
checks human/JSON rejection parity and rejects malformed responses without
fallback or mutation.

The formatter exposed an O(source-bytes times token-count) runtime cost:
`strings.slice` revalidated the complete UTF-8 source for every token span.
Both C backends now use the existing valid-string invariant and validate only
string layout plus slice boundaries, as `byteLen` and `byteAt` already do.
UTF-8 boundary rejection, allocation accounting, failure injection and
sanitizer gates remain unchanged. On the development reference machine, the
largest compiler source format check fell from tens of seconds to below one
second; this is evidence for the algorithmic fix, not a portable threshold.
Canonical acceptance, the default-transition differential, the complete
Stage1/Stage2 fixed point and release-archive smoke all pass with this boundary.

P179c2 moves project test inventory, exact/all selection, test lowering and
shared native runner generation through `mlgc`. The Mallang compiler emits a
length-framed `TEST|1` response with one source/span/encoded-ID `CASE` record
per selected test and the complete generated C payload. Rust remains the
filesystem, process and diagnostic-replay host; it validates every count,
source boundary, UTF-8 span, test-root path, ID and payload byte before writing
or executing the runner. `TERR` selection failures and malformed protocol
responses never fall back to Stage0 or mutate the prior runner.

The representative project test corpus required the self-hosted C backend to
close its closure environment/call/drop ABI, intrinsic function-value thunks
and the `Map[K,V]` layout plus `newMap`, `count` and `insert` runtime. The
Stage0/self workflow covers passing, failing, preflight, exact, unknown and
empty suites, shared-runner allocation accounting, strict C11, ASan/UBSan and
protocol routing. Local acceptance covers the canonical 591 library, 18 driver
and four hardening tests; thirteen positive, nine runtime-rejection and zero
boundary B3 paths; the default-transition gate; an 11,186,276-byte fixed point
with 478 compiler-pair tasks, 173 parser sources, fourteen backend fixtures,
eight backend projects, twenty-one native pairs and sanitizer regeneration;
and the release-archive smoke.

P179c3 moves standalone and project `build`/`run` C generation through
`mlgc native-build*`/`native-run*`. Successful responses use the strict
length-framed `NATIVE|1|<mode>|<bytes>` protocol; the Rust host validates the
mode, byte length and complete C payload before writing or invoking `clang`.
Backend failures exit through the capability boundary instead of masquerading
as source diagnostics, and malformed responses never fall back or mutate an
existing artifact.

Standalone sources without imports retain the direct semantic/IR path so
diagnostic ordering remains identical to Stage0. Imported standalone sources
use a synthetic package layout and the shared package/linker/standard path.
The self-hosted backend covers direct and function-value process arguments,
environment lookup, stdin reads and standard `Error`/`ErrorKind` printing.
The public self path passes arguments, environment, stdin/stdout/stderr, exit,
allocation-failure and sanitizer acceptance. Local closure evidence includes
the canonical 591 library, 20 driver and four hardening tests; thirteen
positive, nine runtime-rejection and zero boundary B3 paths; the
default-transition gate; an 11,434,135-byte fixed point with 478 compiler-pair
tasks, 173 parser sources, fourteen backend fixtures, eight backend projects,
twenty-one native pairs and sanitizer regeneration; and the release-archive
smoke. P179d now owns the packaged default switch.

### P179d: Default Switch And Packaging

- make release and install artifacts invoke the self-hosted compiler by default
- package the minimum reproducible bootstrap metadata and document Stage0
  recovery from a clean checkout
- validate macOS arm64 and Linux x86_64 archives, checksums and installed UX
- keep explicit rollback available without network access

The P179d implementation is complete locally. The public driver now selects
self by default and resolves its sibling `mlgc`; `--compiler stage0` is the
explicit in-process recovery/oracle path. Release archives contain both `mlg`
and fixed-point `mlgc`, and the installer validates protocol/version provenance,
the exact archive entry set and checksum before replacing the pair. Local
archive acceptance covers repeated deterministic builds, clean installation,
missing-core failure, explicit Stage0 recovery and offline rollback.

The tracked clean-checkout recovery path is:

```sh
cargo build --locked --bin mlg
scripts/build-self-hosted-compiler.sh \
  --stage0 target/debug/mlg \
  --output target/debug/mlgc
```

P179d is complete. The published change passed the default-transition,
fixed-point and release-artifact jobs on macOS arm64 and Linux x86_64. Both
platform archives passed clean installation, explicit Stage0 recovery and
v1.0.0 rollback/current re-upgrade acceptance, and the combined checksum bundle
was produced. P179e owns the remaining v1.2.0 publication and installed-release
verification.

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
- [x] self-hosted public project discovery, diagnostics, check, IR and build
- [x] self-hosted format and project-wide atomic write/check workflow
- [x] self-hosted test selection and runner generation
- [x] self-hosted run and native process workflow
- [x] local complete Stage0/default command and conformance parity
- [x] default release artifacts use the Mallang compiler core
- [x] macOS arm64 and Linux x86_64 packaging and clean-install acceptance
- [ ] B5 publication, signed tag and GitHub Release acceptance

## Excluded

- deleting the Rust Stage0 source or differential oracle
- changing the public `mlg` command solely for implementation provenance
- silently falling back to Rust after a self-hosted compiler failure
- requiring network access or an untracked binary to recover the compiler
- incompatible 2.0 naming or syntax changes
