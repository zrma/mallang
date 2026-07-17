# Mallang v1.1 Streaming Text I/O Acceptance

Status: release-ready; supported-platform acceptance pending

## Canonical command

```sh
scripts/check-v1x-acceptance.sh
```

The command runs the 98-rule v1 conformance map, canonical and optimized
compiler gates, complete generated C sanitizers, deterministic release artifact
and clean-install checks, then the v1.x upgrade rehearsal.

## Compatibility rehearsal

One explicit prefix executes the following sequence:

1. Install the published `v1.0.0` release and run the representative `textstats`
   workflow.
2. Upgrade to the current `v1.1.0` artifact and verify identical existing-v1
   observable output plus the new streaming fixture.
3. Roll back explicitly to published `v1.0.0` and repeat the existing-v1
   workflow.
4. Re-upgrade to `v1.1.0`, repeat both workflows, and finish on the new release.

## Release evidence

- [x] package, compiler, archive, checksum and installer version `1.1.0`
- [x] 98 rules, 23 profiles and 65 evidence entries complete
- [x] bounded-memory UTF-8 line semantics and recoverable failure injection
- [x] strict C, ASan/UBSan and zero live-allocation checks
- [x] v1.0.0 upgrade, rollback and v1.1.0 re-upgrade compatibility rehearsal
- [ ] macOS arm64 and Linux x86_64 full acceptance
- [ ] combined checksum bundle and clean public installation
- [ ] signed `v1.1.0` tag and public stable GitHub release
