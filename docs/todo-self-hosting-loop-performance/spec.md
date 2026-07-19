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
  project suite to the full gate.
- `scripts/check-self-hosting-lexer.sh --focus <area>` is the edit-loop gate for
  `lexer`, `parser`, `packages`, `linker`, `specialize`, `semantic`, `ir` or
  `standard`. It runs two or three exact compiler tests, representative
  differential fixtures and one sanitizer path for only that ownership area.
- `--jobs <count>` and `SELF_HOSTING_JOBS` control bounded fixture/corpus
  concurrency. The default uses available processors but is capped at four.
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

## 2026-07-19 Acceleration Result

Sampling the generated Stage1 on the 227 KB semantic source attributed 8,220 of
8,223 samples to repeated UTF-8 scalar counting. `strings.byteLen` and
`strings.byteAt` validated the entire string for every byte cursor operation,
turning parser traversal into quadratic work. Separating constant-time string
layout validation from full UTF-8 validation reduced the direct Stage1 parse
from about 129 seconds to 4.25 seconds. A complete twelve-source
`augment-project` run that had exceeded fifteen minutes now completes in about
9.6 seconds and matches Stage0 byte-for-byte.

The gate now runs independent fixture and parser-corpus differentials with up
to four workers. `mlg test` generates artifacts deterministically, compiles up
to four generated C test programs concurrently and still reports the
lowest-index failure first. CI runs the canonical core and deep sanitizer gate
once; platform artifact jobs only repeat platform-specific release acceptance.

On the same local host class, representative focused gates complete in 38-46
seconds, the fast gate completes in 101 seconds, and the full 263-test,
167-source gate completes in 375 seconds. The full path is about 6.2x faster
than the preceding 2,317-second observation while preserving full publication
coverage. The twelve-source compiler `link-project`, `prepare-project` and
`check-project` outputs also match Stage0 and execute concurrently in the full
gate.

The remaining dominant phase is the 307-second bootstrap. Of that phase, about
250 seconds comes from emitting and compiling 263 complete generated C test
programs totaling roughly 1.76 GB. The next high-leverage optimization is one
shared compiler object plus small per-test harnesses, or an equivalent single
test-runner binary. Incremental compiler caching is lower priority until this
whole-program duplication is removed.
