# Completion Evidence: v0.6 Standard Library

상태: local acceptance complete; Ubuntu CI confirmation pending publication

## Implemented surface

- Compiler-owned `std/errors`, `std/fs`, `std/io`, `std/os`, `std/strings`, and
  `std/collections` packages in project and standalone mode.
- Typed direct and function-value intrinsic lowering with ordinary ownership,
  argument-mode, visibility, and explicit-specialization checks.
- UTF-8 text/conversion, process state, standard streams, text files, standard
  errors, and opaque owned `Map[K,V]` runtime behavior.
- Multi-package `examples/projects/textstats` reference CLI with explicit
  `Result` handling, stderr, and process exit behavior.
- Public API and semantics in `docs/STANDARD_LIBRARY.md` and `SPEC.md`.

## Local acceptance

The repository-owned local gates pass on the supported macOS arm64 host:

| Gate | Evidence |
| --- | --- |
| `scripts/check.sh` | 526 unit tests, clippy, all examples, strict generated C, runtime failure and sanitizer harnesses |
| `scripts/check-collections-map-runtime.sh` | ownership, growth, callbacks, specialization thunks, allocation accounting/failure injection, ASan/UBSan |
| `scripts/check-reference-cli.sh` | stdout/file success, usage, missing/invalid input, write failure, zero live allocations, strict C, ASan/UBSan |
| `scripts/check-release-binary.sh` | optimized compiler binary rebuilds and runs process/file/Map/reference-CLI programs |
| `scripts/check-generated-c-sanitizers.sh --assume-generated` | 67 generated C programs pass the deep sanitizer sweep |

The repository publication diff gate also passes for the P153 candidate.

## Platform matrix

| Platform | Status | Gate |
| --- | --- | --- |
| macOS arm64 | verified locally | repository check, release binary, strict C, ASan/UBSan |
| Ubuntu Linux x86_64 | configured, confirmation pending | `.github/workflows/ci.yml` runs `scripts/check.sh` on `ubuntu-latest` |

The Ubuntu row is not considered observed until the P153 candidate is published
and its GitHub Actions run succeeds. A local non-native emulation result is not
used as a substitute for that CI evidence.

## Remaining publication gate

1. Publish the approved local stack without changing the v0.6 implementation.
2. Confirm the repository CI run succeeds on `ubuntu-latest`.
3. Update this evidence and the P153 status to complete.

No version bump, tag, release, or package publication is part of this gate.
