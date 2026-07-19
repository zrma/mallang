# Spec: Self-Hosting Loop Performance

Status: complete (2026-07-18)

## Problem

The B2 differential gate grew to roughly fifty minutes on a local supported
host. Rust compilation is not the dominant cost. The generated Stage1,
allocation-accounting Stage1 and ASan/UBSan Stage1 were compiled without C
optimization and each reparsed the complete 159-source repository corpus.

The largest compiler source is about 205 KB. A single unoptimized pass through
that source took minutes, so running the same work in normal, accounting and
sanitizer configurations dominated every B2 iteration.

## Contract

- `scripts/check-self-hosting-lexer.sh` without arguments remains the canonical
  full gate. It runs every focused fixture and all discovered repository sources
  through generated Stage1, strict allocation accounting and ASan/UBSan.
- `scripts/check-self-hosting-lexer.sh --fast` is an inner-loop gate. It keeps
  complete Stage0/Stage1 differential coverage, runs focused fixtures through
  strict allocation accounting, runs one project test and sanitizer smoke for
  each lexer, parser, semantic and typed-IR boundary, and leaves the complete
  209-test project suite to the full gate.
- The fast gate is not milestone, publication or release evidence. A logical
  B2 change still requires the full gate before publication.
- Stage1 and strict-accounting native programs use strict C11 with `-O2`.
  Sanitizer programs use strict C11 with `-O1`, ASan/UBSan and frame pointers.
- Optimization must not change generated C identity, normalized output,
  diagnostics, allocation accounting or sanitizer cleanliness.

## Initial Evidence

Using the same generated C and the largest compiler semantic source, optimized
Stage1 took 129.09 seconds and optimized strict accounting took 128.67 seconds.
The optimized sanitizer path took 390.82 seconds. All three outputs were
byte-identical to the Rust Stage0 oracle, and the accounting and sanitizer runs
emitted no diagnostic output.

These wall times are observational and host-dependent. They justify the
optimization profile but are not compatibility promises or CI thresholds.

## Acceptance

- [x] preserve the argument-free full-gate contract
- [x] add an explicit `--fast` inner-loop profile
- [x] keep complete Stage0/Stage1 repository differential coverage in fast mode
- [x] retain focused allocation accounting and representative ASan/UBSan smoke
- [x] retain one exact project test per compiler phase in fast mode
- [x] compile generated self-hosting binaries with strict optimized C flags
- [x] pass shell syntax, fast, full, Rust and documentation gates
- [x] record the resulting full-gate wall-time observation

## Result

The fast gate completed in 208 seconds. The optimized full gate completed in
1,147 seconds while retaining 209 project tests, 159 repository parser sources,
strict accounting and ASan/UBSan. The immediately preceding unoptimized full
gate took 3,211 seconds on the same host, so the full path improved by about
2.8x and the inner loop by about 15.4x.

These observations compare adjacent repository revisions on one host. They are
evidence that the loop bottleneck was removed, not portable performance
thresholds.

## Follow-Up Observation

P176e4c1 increased the discovered parser corpus to 167 sources. The optimized
fast gate completed in 392 seconds and the complete Stage1,
strict-accounting and ASan/UBSan gate completed in 2,317 seconds on the same
class of local host. A direct Stage1 `augment-project` run over the complete
twelve-file compiler source set still had not completed after fifteen minutes
and was stopped. The standard augmentation arena copy is flat, so the remaining
cost is in the combined multi-source parse/link/normalize path. This is tracked
as residual performance debt and is not recorded as a successful augmentation
result.
