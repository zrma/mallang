# P170: Representative Dogfood

상태: complete (2026-07-17)

## Scope

Frozen v1 candidate surface를 사용하는 `examples/projects/textstats`를 published artifact와
같은 release archive에서 clean install한 `mlg`로 검증한다. 검증은 source를 별도 clean work
directory에 복사하고 동일한 format, check, test, build, run workflow를 두 번 수행한다.

이 milestone은 새 syntax, type, ownership 또는 standard-package API를 추가하지 않는다.

## Executable evidence

Standalone acceptance entrypoint는 다음 명령이다.

```sh
scripts/check-v09-dogfood.sh
```

이 명령은 deterministic release archive를 생성하고 clean prefix에 설치한 뒤 다음 contract를
검증한다.

1. copied project에는 ignored build output이 포함되지 않는다.
2. `fmt --check`와 `fmt`가 canonical source를 변경하지 않는다.
3. `check`와 project test discovery/output이 두 실행에서 동일하다.
4. native `build` 결과와 `mlg run` 결과가 UTF-8 reference input에 대해 동일하다.
5. stdout, output-file mode와 usage exit status가 두 실행에서 동일하다.
6. generated C가 두 실행에서 byte-identical하다.
7. generated C와 binary가 strict C, ASan/UBSan, allocation accounting과 reference CLI
   success/error-flow gate를 통과한다.

Canonical repository gate는 P160 clean-install compiler를 재사용해 중복 archive build 없이
같은 workflow를 실행한다.

```sh
scripts/check-v09-dogfood.sh \
  --compiler target/mallang/release-artifact-smoke/home/.local/bin/mlg
```

## Findings

| Class | Finding | Resolution |
| --- | --- | --- |
| compiler bug | 없음 | frozen compiler behavior를 변경하지 않았다. |
| diagnostic | 없음 | 성공/실패 stdout, stderr와 exit status가 기존 contract와 일치했다. |
| documentation | 없음 | public command와 expected behavior가 current implementation과 일치했다. |
| test/maintenance gap | representative production source 두 파일이 formatter canonical form이 아니었다. | current formatter로 정규화하고 no-write/idempotence를 dogfood gate에 고정했다. |
| test/maintenance gap | `textstats` project에 project test가 없었다. | UTF-8 byte/scalar/line/map summary를 검증하는 package test를 추가했다. |
| test/maintenance gap | generic test-workflow gate가 `textstats`를 empty-suite fixture로 재사용했다. | dedicated `project-test-empty` fixture로 분리해 representative test와 empty-suite contract를 독립시켰다. |

두 gap 모두 observable language surface, compatibility contract 또는 standard API 변경을
요구하지 않았다. P170 종료 시 unresolved release blocker는 없다.

## Completion

- [x] clean release archive installation
- [x] clean representative source copy
- [x] repeated format/check/test/build/run workflow
- [x] deterministic output and generated C
- [x] strict C, sanitizer and allocation-accounting reference CLI gate
- [x] issue classification without frozen-surface change
- [x] canonical `scripts/check.sh` integration

다음 milestone은 freeze 이후 change audit, conformance completeness와 supported-platform
artifact를 검증하고 `v0.9.0`을 게시하는 P171이다.
