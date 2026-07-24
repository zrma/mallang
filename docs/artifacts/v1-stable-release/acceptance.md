# Mallang v1.0.0 Stable Acceptance

Status: complete; released as v1.0.0 on 2026-07-17

## Frozen contract

The stable release adds no language or compiler feature after v0.9.0.
`scripts/check-v1-stable-acceptance.sh` requires `v1.0.0-rc.1` to be an ancestor
and rejects any difference from v0.9.0 under `src/` or
`docs/conformance/v1-rules.json`. The existing freeze and 98-rule conformance
gates then run unchanged.

## Canonical command

```sh
scripts/check-v1-stable-acceptance.sh
```

The command covers canonical and optimized compiler tests, release binary,
complete generated C sanitizers, deterministic artifacts and clean installation.
It then runs `scripts/check-v1-stable-rehearsal.sh --reuse-release-artifact`.

## Upgrade and rollback

One explicit prefix executes the following sequence:

1. Install the published `v1.0.0-rc.1` release over HTTPS and verify its checksum.
2. Upgrade to the current `v1.0.0` artifact through the offline release path.
3. Roll back explicitly to published `v1.0.0-rc.1`.
4. Re-upgrade to `v1.0.0` and finish on stable.

Every phase runs a clean-copy `textstats` format, check, test, build, direct run,
`mlg run`, output-file, strict C, sanitizer and allocation-accounting workflow.
Program output, test output, usage failure and summary output remain byte-identical.

## Release evidence

- [x] package, compiler, archive, checksum and installer version `1.0.0`
- [x] compiler source and conformance map unchanged from v0.9.0
- [x] 98 rules, 23 profiles and 64 evidence entries complete
- [x] RC upgrade, rollback and stable re-upgrade
- [x] macOS arm64 and Linux x86_64 full acceptance
- [x] combined checksum bundle and clean public installation
- [x] signed `v1.0.0` tag and public stable GitHub release
- [x] stable compatibility and private vulnerability reporting policy
