# P172: v1 RC and Rollback Rehearsal

상태: complete; released as v1.0.0-rc.1 prerelease (2026-07-17)

## Canonical command

```sh
scripts/check-v1-rc-acceptance.sh
```

이 명령은 P171의 freeze, conformance, canonical/optimized compiler와 complete generated C
sanitizer acceptance를 먼저 실행한 뒤 다음 rehearsal을 추가한다.

```sh
scripts/check-v1-rc-rehearsal.sh --reuse-release-artifact
```

## Prerelease distribution

Release version parser는 기존 `major.minor.patch`와 SemVer prerelease suffix가 붙은
`major.minor.patch-prerelease`를 허용한다. P172의 exact candidate는 `1.0.0-rc.1`이다.

- deterministic archive root와 filename
- checksum filename parser와 all-target bundle
- online/offline installer URL, archive shape와 staged binary version
- compiler `--version`과 package version

모든 위치가 같은 exact version을 사용한다. Empty, repeated-dot 또는 invalid-character
prerelease는 download/build 전에 거부된다.

## Upgrade and rollback sequence

같은 explicit prefix에서 다음 sequence를 실행한다.

1. Published `v0.9.0` release를 HTTPS/checksum 경로로 설치한다.
2. Current supported-host `v1.0.0-rc.1` archive를 clean build하고 offline 설치로 upgrade한다.
3. Published `v0.9.0`을 다시 online 설치해 explicit rollback한다.
4. 같은 RC archive를 다시 설치해 re-upgrade한다.

각 단계는 compiler version을 확인하고 별도 clean-copy `textstats`에서 format, check, test,
build, direct run, `mlg run`, output-file, usage failure, strict C, ASan/UBSan와 allocation
accounting을 두 번 실행한다. v0.9 baseline과 upgrade/rollback/re-upgrade 단계의 program output,
test output, error output과 output file이 byte-identical해야 한다.

## Platform contract

GitHub Actions macOS arm64와 Ubuntu Linux x86_64 release matrix가 같은 full P172 acceptance를
실행하고 target archive 및 combined checksum bundle을 생성한다. RC tag/release는 두 matrix와
bundle이 모두 green일 때만 게시한다.

## Acceptance result

- [x] `1.0.0-rc.1` archive, checksum and installer support
- [x] malformed prerelease rejection before I/O
- [x] clean RC install and representative project
- [x] published v0.9.0 to RC same-prefix upgrade
- [x] explicit RC to v0.9.0 rollback
- [x] RC re-upgrade and final version state
- [x] cross-version representative observable-output identity
- [x] canonical, optimized and complete generated C sanitizer acceptance
- [x] macOS arm64 and Linux x86_64 target archive matrix
- [x] signed prerelease tag and public GitHub prerelease

P172 종료 시 unresolved v1 blocker는 없다. 다음 milestone은 frozen contract를 변경하지 않고
stable package/version, final audit, artifact와 release provenance를 닫는 v1.0.0 release다.
