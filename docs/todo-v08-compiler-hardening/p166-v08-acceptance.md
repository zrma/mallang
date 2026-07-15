# P166: v0.8 Compiler Hardening Acceptance

상태: complete; released as v0.8.0 (2026-07-16)

## Canonical command

```sh
scripts/check-v08-acceptance.sh
```

이 명령은 다음 evidence를 순서대로 구성한다.

1. `scripts/check.sh`: Rust tests, deterministic properties, debug CLI crash corpus,
   full example check/build/run, warning-clean C, focused sanitizers, generated C identity,
   release archive/install과 clean-project workflow
2. `scripts/check-release-binary.sh`: optimized `mlg`의 CLI, diagnostics, parser recovery,
   crash corpus, ownership rejection, standard library와 native workflow
3. `scripts/check-generated-c-sanitizers.sh --assume-generated`: canonical example set 전체의
   native output과 ASan/UBSan output identity

`scripts/verify-v0-rc.sh`는 이 acceptance를 release helper, roadmap, local-stack와 attribution
검사에 연결한다. `--skip-deep-sanitizers`는 명시적인 local fast path일 뿐 publication evidence가
아니다.

## Platform contract

GitHub Actions release matrix는 macOS arm64와 Ubuntu Linux x86_64에서 동일한
`scripts/check-v08-acceptance.sh`를 실행한다. 각 job이 만든 target-named archive만 checksum
bundle input이 된다. Matrix 또는 checksum bundle이 실패하면 v0.8 tag/release를 만들지 않는다.

## Acceptance result

- [x] user-reachable crash corpus returns non-zero stage-owned JSON diagnostics
- [x] debug and optimized release compiler diagnostic parity
- [x] 575 library tests, 2 CLI tests and 4 deterministic hardening properties
- [x] every canonical example emits warning-clean C and expected native output
- [x] complete generated C set matches native output under ASan/UBSan
- [x] generated C and release archive repeated-build byte identity
- [x] installed release compiler clean-project format/check/test/build/run
- [x] macOS arm64 and Linux x86_64 CI acceptance and checksum bundle
- [x] public repository publication boundary and private-inventory gate
- [x] v0.9 language-freeze decision gate synchronized

No parser-library migration, full LSP, new ownership syntax or language feature was needed to close
v0.8. Numerical performance thresholds remain observational under the approved Q4 second decision.
