# P159 Release Artifacts and Installation

상태: implementation complete; published native CI pending

## Goal

Mallang release binary를 supported host에서 재현 가능한 archive로 만들고, release에 함께
게시되는 checksum을 검증한 뒤 clean prefix에 설치하거나 교체할 수 있게 한다. Source checkout과
Rust toolchain 없이도 설치된 `mlg`가 representative project의 check/build/run/test workflow를
수행해야 한다.

## Supported Targets

P159가 지원하는 target은 다음 둘뿐이다.

| host | target triple | artifact |
| --- | --- | --- |
| macOS arm64 | `aarch64-apple-darwin` | `mallang-v<version>-aarch64-apple-darwin.tar.gz` |
| Linux x86_64 glibc | `x86_64-unknown-linux-gnu` | `mallang-v<version>-x86_64-unknown-linux-gnu.tar.gz` |

Artifact build와 install은 `uname`으로 현재 host를 판별한다. 다른 OS/architecture, cross
compilation과 target override는 명시적으로 거부한다.

## Archive Contract

`<version>`은 leading `v`가 없는 `major.minor.patch`이고 build하는 Cargo package version 및
`mlg --version`과 같아야 한다. Archive는 target별로 다음 exact layout을 가진다.

```text
mallang-v<version>-<target>/
mallang-v<version>-<target>/bin/
mallang-v<version>-<target>/bin/mlg
mallang-v<version>-<target>/LICENSE-MIT
mallang-v<version>-<target>/LICENSE-APACHE
mallang-v<version>-<target>/README.md
```

- `bin/mlg` mode는 `0755`, 나머지 regular file은 `0644`, directory는 `0755`다.
- Tar entry 순서, uid/gid, owner/group name과 mtime을 정규화하고 gzip timestamp를 0으로
  고정한다. 같은 source와 native binary로 반복한 build는 byte-identical해야 한다.
- Symlink, absolute path, parent traversal과 추가 entry는 허용하지 않는다.
- Project license expression은 `MIT OR Apache-2.0`이며 두 license text를 archive에 함께 넣는다.
- `README.md`는 compiler prerequisite와 설치된 binary의 기본 확인 방법을 설명한다.

## Checksum Contract

Release root의 `SHA256SUMS`는 두 target archive를 filename byte order로 정렬해 다음 형식으로
기록한다.

```text
<lowercase-64-hex><two spaces><archive-filename>
```

Version과 target이 중복되거나, 두 supported target 중 하나가 빠지거나, 예상하지 않은 archive
이름이 있으면 release bundle assembly는 실패한다. Checksum은 transport corruption 검출이며
별도 artifact signing을 대신하지 않는다.

## Installer Contract

Public installer interface는 다음과 같다.

```text
./install.sh --version <major.minor.patch> [--bin-dir <directory>]
./install.sh --version <major.minor.patch> [--bin-dir <directory>] \
  --archive <path> --checksums <path>
```

- `--version`은 필수다. Latest lookup과 implicit update는 하지 않는다.
- 기본 install directory는 `$HOME/.local/bin`이고 `--bin-dir`로 명시적으로 바꿀 수 있다.
- 일반 mode는 `https://github.com/zrma/mallang/releases/download/v<version>/`에서 현재
  host archive와 `SHA256SUMS`를 HTTPS로 받는다.
- `--archive`와 `--checksums`는 offline/local verification을 위해 함께만 사용할 수 있다.
- Installer는 `curl` 또는 local input, `tar`, `awk`, `sort`, `cmp`, SHA-256 tool과 `clang`을
  사전 검사한다. `clang`은 설치된 `mlg build`, `mlg run`, `mlg test`가 generated C를 native
  binary로 만들기 위한 runtime prerequisite다.
- Exact checksum과 archive entry set을 검증한 뒤 temporary file의 `mlg --version`을 확인하고
  destination의 `mlg`를 atomic rename으로 교체한다.
- 같은 command에 새 explicit version을 전달하는 것이 v0.7 update workflow다. 별도 self-update
  command, privilege escalation과 shell profile 수정은 하지 않는다.

## Build And CI Contract

- `scripts/build-release-artifact.sh`는 current native target의 optimized locked binary를 만들고
  version을 확인한 뒤 하나의 deterministic archive를 출력한다.
- `scripts/write-release-checksums.py`는 one-target local smoke 또는 exact two-target release
  bundle의 `SHA256SUMS`를 만든다.
- `scripts/check-release-artifacts.sh`는 archive를 두 번 만들어 byte identity를 확인하고,
  checksum tamper rejection, clean temporary prefix install/reinstall, version/help와 copied
  representative project check/build/run/test를 검증한다.
- GitHub Actions는 `macos-15` arm64와 `ubuntu-latest` x86_64 native runner에서 같은 smoke를
  실행한다. Linux는 `clang`을 설치하고 macOS는 available `clang`을 확인한다.
- Matrix output을 합치는 job은 exact two-target checksum bundle과 installer를 workflow
  artifact로 보존한다. Tag와 GitHub Release asset publication은 P159에서 자동화하지 않는다.

## Acceptance

- [x] dual-license metadata와 archive license payload
- [x] supported-host detection과 exact artifact naming
- [x] deterministic archive layout 및 repeated-build byte identity
- [x] exact one/two-target checksum generation과 malformed bundle rejection
- [x] HTTPS/offline installer, checksum 및 archive-shape verification
- [x] default/explicit prefix atomic install과 explicit-version reinstall
- [x] installed binary version/help 및 project check/build/run/test smoke
- [x] macOS arm64/Linux x86_64 native CI artifact matrix와 combined bundle 정의
- [x] local canonical verification 및 public documentation synchronization
- [ ] published native CI matrix와 combined bundle success 확인

## Excluded

- Windows와 Linux musl
- Cross compilation과 user-selectable target override
- Homebrew, apt, cargo install과 package registry publication
- Code signing, notarization, provenance attestation과 artifact signature
- Automatic latest-version lookup, self-update command와 rollback manager
- Tag, GitHub Release와 release asset publication
