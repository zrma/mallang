# Completion Evidence: v0.6 Standard Library

상태: complete

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

The repository publication push gate passed for the P153 stack.

## Platform matrix

| Platform | Status | Gate |
| --- | --- | --- |
| macOS arm64 | verified locally | repository check, release binary, strict C, ASan/UBSan |
| Ubuntu Linux x86_64 | verified in CI | `.github/workflows/ci.yml` runs `scripts/check.sh` on `ubuntu-latest` |

GitHub Actions `CI` run `29358952361` passed on published commit
`e61ac8f4509c968b2552d92ba08ed0776a2d30f1`. The successful `check` job ran the
canonical repository gate on an Ubuntu Linux x86_64 runner. A local non-native
emulation result was not used as a substitute for this evidence.

## Completion result

The approved implementation stack is published, the local macOS arm64 gates
pass, and the Ubuntu Linux x86_64 CI gate passes. P153 is complete.

No version bump, tag, release, or package publication was part of this gate.
