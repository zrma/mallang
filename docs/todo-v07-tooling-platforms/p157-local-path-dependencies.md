# P157 Local Path Dependencies

상태: complete

## Goal

`mallang.toml`이 선언한 local project를 하나의 deterministic compilation graph로
연결한다. Registry, network resolution, version solver와 lockfile 없이 cross-project public
package API를 기존 linker, ownership, specialization, IR/backend 경로로 재사용한다.

## Manifest Contract

```toml
[project]
name = "app"

[dependencies]
text = { path = "../text" }
```

- `[dependencies]`는 optional table이며 key는 dependency manifest의 `[project].name`과
  정확히 같아야 한다. Alias는 없다.
- 각 entry는 exact `path` field 하나만 가진다. Path는 declaring manifest directory 기준의
  non-empty relative directory path여야 하며 absolute path와 direct manifest-file path는
  거부한다.
- Path는 filesystem canonicalization 뒤 identity가 된다. 같은 canonical project는 diamond
  graph에서 한 번만 load한다.
- 서로 다른 canonical project가 같은 project name을 쓰는 경우와 dependency project root가
  다른 graph member의 `src/` 또는 `tests/` 아래에 겹치는 경우를 거부한다.

## Graph And Import Contract

- Dependency key의 lexical order로 DFS하고 dependency-first postorder로 source를 load한다.
  Root project source는 항상 마지막이다.
- Project cycle은 package parsing 전에 project name chain으로 거부한다.
- Package identity는 기존 `<project-name>` 또는 `<project-name>/<directory>`를 유지한다.
- 한 project source는 own package 또는 manifest에 직접 선언한 dependency package만 import할
  수 있다. Transitive project를 직접 쓰려면 root manifest에도 직접 dependency로 선언한다.
- Same-project package import와 compiler-owned `std/...` import behavior는 바꾸지 않는다.
- Dependency의 `src/main.mlg`는 application entrypoint이므로 dependency compilation source에서
  제외한다. 같은 dependency root package의 다른 `.mlg` source와 nested package는 public API로
  import할 수 있다.
- Dependency `tests/`는 consumer graph에 포함하지 않는다. Root project tests만 load하며 public
  dependency API를 ordinary import로 사용할 수 있다.

## Library And Command Contract

- `src/`는 계속 필수지만 root `src/main.mlg`는 project discovery 자체의 필수 조건이 아니다.
- `mlg check`, `mlg fmt`와 `mlg test`는 entrypoint 없는 library project를 지원한다.
- `mlg test`는 library test body를 synthetic native `main`으로 lowering한다. Empty test suite도
  기존 `0 passed; 0 failed` contract를 유지한다.
- `mlg build`와 `mlg run`은 root `src/main.mlg`가 없으면 native compilation 전에 stable
  project diagnostic으로 거부한다.
- `mlg fmt`는 요청한 root project의 `src/`와 `tests/`만 변경한다. External dependency source를
  format하지 않는다.

## Excluded

- Registry, URL/git dependency, version requirement, feature flag와 lockfile
- Dependency alias, workspace manifest와 source vendoring
- Dependency tests 실행과 dependency application `main` 실행
- Build cache 또는 incremental dependency artifact

## Acceptance Matrix

- [x] manifest parse, relative path, key/name and unknown-field diagnostics
- [x] deterministic transitive/diamond discovery and canonical deduplication
- [x] project name collision, graph cycle and overlapping source-root rejection
- [x] direct dependency import boundary and transitive import rejection
- [x] cross-project public/private API and generic/recursive owned value linking
- [x] dependency `src/main.mlg` and `tests/` exclusion
- [x] library project check/fmt/test and build/run entrypoint diagnostics
- [x] root tests importing dependency APIs with synthetic native execution
- [x] strict C, zero-allocation, ASan/UBSan and debug/release CLI smoke
- [x] README/SPEC/roadmap/handoff synchronization

## Approval Boundary

P154 Q3의 local-relative dependency, exact project-name key, library project 및 no-registry
경계는 2026-07-15 사용자 승인으로 확정됐다. 이 문서는 그 승인안을 기존 compiler 구조에
맞는 deterministic source/import contract로 구체화한다. 새로운 package naming syntax나
network dependency가 필요해질 때만 별도 decision gate를 연다.

## Completion Evidence

- Project unit tests cover relative-path validation, exact key/name matching, transitive diamond
  order/deduplication, graph cycle, name collision과 overlapping source boundary rejection.
- Package/compiler tests cover direct dependency imports, undeclared transitive rejection,
  public/private visibility, generic specialization and library test lowering without app `main`.
- `scripts/check-path-dependencies.sh` exercises root-only formatting, dependency-first app
  check/build/run/test, dependency entrypoint/test exclusion와 library check/test/build/run boundary.
- The app and root/library test generated C pass zero-allocation wrappers, strict C and
  ASan/UBSan under both debug and optimized release `mlg` binaries.
