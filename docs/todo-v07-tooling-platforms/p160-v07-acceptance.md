# P160: v0.7 Acceptance

상태: implementation in progress

## Canonical workflow

`scripts/check-v07-acceptance.sh`는 repository fixture를 복사하지 않고 빈 ignored work
directory에 다음 두 project를 생성한다.

- `toolkit`: executable entrypoint가 없는 local library project
- `workflow`: `toolkit`을 manifest-relative path dependency로 사용하는 executable project

Acceptance는 deterministic release archive를 만들고 clean default prefix에 설치한 `mlg`만
사용해 다음 순서를 검증한다.

1. unformatted library/application/test source에 대한 no-write `fmt --check` failure
2. canonical `fmt`, clean `fmt --check`와 repeated `fmt` byte identity
3. library human check와 application JSON-mode successful check
4. dependency-backed isolated project test
5. explicit output path의 native build와 direct binary execution
6. installed compiler의 project run

여기서 release build는 optimized Rust compiler가 들어 있는 target-named release archive의
설치본으로 Mallang project를 native build하는 경로다. Mallang source profile flag는 아직
public CLI contract가 아니므로 별도의 `--release` option을 암시하지 않는다.

## Platform gate

- Local canonical `scripts/check.sh`는 이 acceptance를 포함한다.
- GitHub Actions release matrix는 macOS arm64와 Linux x86_64에서 같은 script를 실행한다.
- Matrix output archive는 기존 combined checksum/install bundle job의 input으로 유지한다.
- `clang`은 native build/run/test를 위한 명시적 runtime prerequisite다.

## Completion checklist

- [x] clean project creation without checked-in project fixture reuse
- [x] installed release compiler format/check/test/build/run workflow
- [x] formatter no-write, deterministic order and byte idempotence
- [x] local path dependency and library command boundary
- [x] successful machine-readable diagnostic mode compatibility
- [x] local canonical and publication boundary gates
- [ ] published macOS arm64/Linux x86_64 CI evidence
- [ ] user-facing documentation and v0.8 decision gate synchronization
