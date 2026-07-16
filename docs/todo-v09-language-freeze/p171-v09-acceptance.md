# P171: v0.9 Acceptance and Release

상태: complete; released as v0.9.0 (2026-07-17)

## Canonical command

```sh
scripts/check-v09-acceptance.sh
```

이 명령은 다음 evidence를 순서대로 구성한다.

1. `scripts/check-v09-freeze.py`: signed `v0.8.0` base 이후 compiler `src/` 변경이 없고
   모든 변경이 documentation, conformance, dogfood 또는 release class에 속하는지 검사한다.
2. `scripts/check-v1-conformance.py`: 98개 normative rule, 23개 profile과 64개 evidence
   item의 exact-set 및 repository path/test symbol 정합성을 검사한다.
3. `scripts/check-v08-acceptance.sh`: canonical repository check, optimized release compiler와
   complete generated C sanitizer sweep을 재사용한다.

`scripts/verify-v0-rc.sh`와 release-artifact CI matrix는 이 P171 entrypoint를 호출한다.
`--skip-deep-sanitizers`는 publication evidence가 아닌 명시적인 local fast path다.

## Freeze change audit

`v0.8.0` 이후 P167-P171 변경은 다음 class로 한정됐다.

| Class | Allowed work |
| --- | --- |
| documentation | stale Copy/ADT/match 교정, rule-indexed candidate contract, compatibility, migration, roadmap와 release record |
| conformance | exact rule map/checker, migration fixtures, dedicated empty-suite fixture와 canonical gate integration |
| dogfood | `textstats` formatter normalization, UTF-8 package test와 clean-install repeated workflow |
| release | version metadata, v0.9 acceptance helpers와 supported-platform CI invocation |

Compiler `src/` diff는 0건이다. P167의 `SPEC.md` 수정은 이미 구현된 Copy classification,
user enum과 nested pattern을 문서에 반영한 correction이며 source behavior 변경이 아니다.

## Platform contract

GitHub Actions release matrix는 full history와 signed base tag를 checkout하고 macOS arm64와
Ubuntu Linux x86_64에서 같은 `scripts/check-v09-acceptance.sh`를 실행한다. 각 job의
target-named archive만 checksum bundle input이 된다. Matrix 또는 checksum bundle이 실패하면
tag/release를 만들지 않는다.

## Acceptance result

- [x] zero compiler source changes after the v0.8 freeze base
- [x] all post-v0.8 changes classified as documentation, conformance, dogfood or release
- [x] 98 normative rules mapped exactly once across 23 profiles and 64 evidence items
- [x] canonical and optimized release compiler acceptance
- [x] repeated clean-install `textstats` format/check/test/build/run
- [x] complete generated C ASan/UBSan native-output identity
- [x] deterministic target archives and combined checksum bundle
- [x] macOS arm64 and Linux x86_64 release artifact matrix
- [x] public repository publication and private-inventory gates
- [x] signed `v0.9.0` tag and GitHub binary release

No new language feature was required to close v0.9. The next milestone is P172 v1 RC clean
install, v0.9 upgrade, explicit rollback and representative-project rehearsal.
