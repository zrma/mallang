# Completion Evidence: v0.7 Tooling and Platforms

상태: technical acceptance complete; v0.8 decision approval pending

## Implemented surface

- Canonical comment-preserving formatter and no-write `fmt --check`.
- Project test discovery, whole-suite preflight and isolated native test processes.
- Manifest-relative local path dependencies and entrypoint-free library workflow.
- Versioned human/JSON diagnostic model and JSONL consumer.
- Deterministic macOS arm64/Linux x86_64 release archives, checksums and installer.
- Clean-project format/check/test/build/run acceptance using the installed release compiler.

## Local acceptance

The canonical `scripts/check.sh` gate passed on the supported macOS arm64 host.
It included 559 Rust tests, formatter/test/dependency/diagnostic harnesses, deterministic release
archive installation, clean-project acceptance, native examples, strict generated C,
allocation/failure injection and ASan/UBSan coverage. Warning-level shell lint, Action workflow
lint, whitespace checks and both public repository boundary gates also passed.

## Published platform matrix

GitHub Actions `CI` run `29433381232` passed on published commit
`75de0edb3459ce9ef4c927302997810573faa699`.

| Platform/job | Status | Evidence |
| --- | --- | --- |
| Ubuntu Linux x86_64 canonical check | passed | repository `scripts/check.sh` |
| Linux x86_64 release artifact | passed | clean-project release acceptance and archive upload |
| macOS arm64 release artifact | passed | clean-project release acceptance and archive upload |
| combined release bundle | passed | two archives, `SHA256SUMS` and `install.sh` |

The downloaded workflow bundle contained exactly the installer, checksum file and both supported
target archives. Both entries passed local SHA-256 verification. The workflow bundle still uses
the current development package version `0.6.0`; the separate v0.7.0 release step owns the version
bump, tag and public binary assets.

## Completion result

The v0.7 implementation and technical acceptance conditions are satisfied locally and on both
supported native CI platforms. P160 remains open only for approval of the proposed v0.8 Q1-Q6
decision gate.

No version bump, tag, GitHub Release or package publication was part of this evidence change.
