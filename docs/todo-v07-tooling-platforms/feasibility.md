# P154 Feasibility Inventory

상태: complete; Q1-Q6 approved on 2026-07-15

이 문서는 v0.6.0 compiler와 repository workflow를 기준으로 v0.7 tooling 구현에
필요한 구조 변경, compatibility 영향과 acceptance 경계를 정리한다. 구현 선택은
`open-questions.md`의 Q1-Q6 승인 뒤 확정한다.

## Baseline

| 영역 | 현재 구조 | v0.7에 필요한 경계 |
| --- | --- | --- |
| source syntax | hand-written lexer/parser, byte `Span` | comment/trivia를 잃지 않는 formatting 전용 lossless stream |
| project | 단일 `mallang.toml`, 단일 `src/`, sorted `.mlg` discovery | tests source set과 recursive local dependency project graph |
| CLI | `src/main.rs`의 수동 subcommand/argument parsing | `fmt`, `test`, diagnostic format option의 공통 parse contract |
| diagnostic | compiler stage/message/optional span 뒤 human string rendering | human/JSON이 공유하는 versioned structured diagnostic |
| native build | Rust `mlg`가 generated C를 `clang`으로 compile | artifact별 compiler prerequisite와 clean-host smoke |
| release | signed source tag/release, binary asset 없음 | two-platform archive, checksum, install/update workflow |
| editor | file-aware single diagnostic, no protocol server | JSON consumer prototype; LSP는 별도 feasibility 이후 판단 |

## Q1 Formatter

현재 lexer의 `skip_ignored`는 whitespace와 `//` comment를 token stream에 넣지 않는다.
AST는 source span을 보존하지만 comment attachment, 원래 string escape spelling과 빈 줄
정보를 소유하지 않는다. AST pretty-printer는 comment를 삭제하므로 사용할 수 없다.

권장 구조:

- parser token과 별도로 raw source slice를 보존하는 lossless formatting token/trivia
  stream을 추가한다.
- non-trivia token의 kind, value와 순서를 바꾸지 않는 것을 semantic preservation
  invariant로 둔다.
- leading/standalone comment는 뒤 token의 nesting 위치에, same-line trailing comment는
  앞 token에 고정한다.
- P155 canonical style은 4-space indent, LF, final newline, 연속 blank line 최대 1개로
  고정하고 line-width wrapping은 제외한다.
- direct `.mlg`와 project input을 지원하며 project file은 deterministic path order로
  처리한다.
- `mlg fmt --check <input>`은 파일을 쓰지 않고 차이가 있으면 non-zero로 종료한다.
- parse 또는 lossless lex에 실패한 파일은 어떤 write도 수행하지 않는다.
- idempotence, comment text 보존, non-trivia token parity를 golden/property regression으로
  검증한다.

호환성 영향: source semantics에는 additive지만 formatter output과 exit status가 public CLI
contract가 된다. Block comment는 현재 언어에 없으므로 P155에서 추가하지 않는다.

## Q2 Test Workflow

현재 compiler는 top-level function만 실행 단위로 가지며 exact `func main()` 하나를
entrypoint로 요구한다. Test declaration, assertion primitive, test registry와 per-test
failure isolation은 없다. Project discovery도 `src/`만 읽는다.

권장 구조:

- `tests/`를 `src/`와 분리된 deterministic source set으로 discovery한다.
- ordinary `func main()`이나 stdout snapshot을 test contract로 재사용하지 않는다.
- P156에서 compiler-owned test declaration, assertion, synthetic runner lowering을 하나의
  syntax/semantic decision으로 확정한다.
- test name/path order를 stable execution order로 사용하고 filter는 substring이 아닌 exact
  또는 explicit pattern contract로 정의한다.
- compile failure는 test execution 전에 종료하고, assertion failure는 test name과 source
  span을 포함하며 전체 process는 non-zero로 종료한다.
- multi-package fixture는 package visibility와 ownership/runtime cleanup을 production compiler
  path와 동일하게 검증한다.

호환성 영향: 새 source location과 declaration surface가 추가된다. 정확한 test/assert syntax는
P156 spec 승인 전까지 고정하지 않는다.

## Q3 Local Path Dependencies

현재 manifest는 `[project].name`만 허용하고 unknown field를 거부한다. `Project`와 package
identity 계산은 모든 source가 한 project의 `src/` 아래 있다고 가정한다.

권장 manifest shape:

```toml
[project]
name = "app"

[dependencies]
text = { path = "../text" }
```

- dependency path는 declaring manifest directory 기준 relative path만 허용한다.
- dependency key는 target manifest의 project name과 같아야 하며 alias는 지원하지 않는다.
- canonical path 중복, project name 충돌과 project-level cycle을 source compilation 전에
  거부한다.
- package identity는 `<project-name>/<package-path>`를 유지한다.
- dependency project source는 deterministic dependency-first order로 load/link한다.
- project discovery와 `mlg check`/`mlg test`는 `src/main.mlg` 없는 library project를
  허용하고 `mlg build`/`mlg run`만 executable entrypoint를 요구한다.
- dependency의 executable entrypoint는 library import 대상에서 제외하고 public package API만
  노출한다.
- registry, network fetch, version solver, lockfile와 transitive source vendoring은 제외한다.

호환성 영향: manifest에 additive field가 생기고 project discovery가 library project를
허용하도록 넓어지지만 import identity와 command별 entrypoint diagnostics는 public contract가
된다. Existing executable project는 그대로 동작해야 한다.

## Q4 Machine-readable Diagnostics

`CompilerError`는 stage/message/optional span을 갖지만 CLI에서 human string으로 변환되며,
`SourceMap`은 start line/column만 render한다. Project/input/CLI/native errors는 compiler stage와
같은 구조를 공유하지 않는다.

권장 JSON Lines record:

```json
{"schema":"mallang.diagnostic.v1","severity":"error","stage":"semantic","message":"...","source":{"path":"src/main.mlg","span":{"byte_start":0,"byte_end":4,"start":{"line":1,"column":1},"end":{"line":1,"column":5}}}}
```

- schema identifier, severity, stable stage, message와 optional source/span을 구조화한다.
- byte offsets는 UTF-8 byte 기준, line/column은 1-based Unicode scalar 기준으로 고정한다.
- human과 JSON renderer는 같은 structured diagnostic을 입력으로 사용한다.
- CLI/input/frontend/package/link/semantic/IR/backend/native stage vocabulary를 고정한다.
- JSON mode는 opt-in이며 diagnostic record는 stderr에 한 줄씩 출력한다.
- JSON string field에 human diagnostic 전체를 감싸는 방식은 금지한다.
- v0.7은 single-error compiler도 허용하고, v0.8 multi-error recovery가 같은 schema에 record를
  추가하도록 설계한다.

호환성 영향: schema와 stage spelling은 versioned machine API다. Human output은 기존 형태를
유지한다.

## Q5 Artifacts And Installation

현재 release는 source-only이며 CI는 Ubuntu에서 canonical repository gate만 실행한다. `mlg`
binary는 실행 중 generated C를 `clang`으로 compile하므로 compiler prerequisite가 artifact
contract에 포함되어야 한다.

권장 contract:

- `mallang-v<version>-aarch64-apple-darwin.tar.gz`
- `mallang-v<version>-x86_64-unknown-linux-gnu.tar.gz`
- `SHA256SUMS`
- archive에는 `bin/mlg`, license와 설치에 필요한 최소 metadata를 넣는다.
- install script는 OS/architecture를 명시적으로 판별하고 checksum 검증 뒤 기본
  `<home>/.local/bin` 또는 explicit `--bin-dir`에 설치한다.
- update는 별도 self-update command 대신 같은 installer에 explicit version을 전달하는
  방식으로 시작한다.
- native GitHub-hosted runner에서 artifact를 만들고 clean temporary prefix에 설치한 뒤
  version/help와 representative project check/build/run/test를 수행한다.
- Linux smoke는 `clang` prerequisite를 명시적으로 설치하고 macOS smoke는 available C compiler를
  확인한다.
- Windows, cross compilation, package manager formula와 crates.io publication은 제외한다.

호환성 영향: archive 이름, checksum file, install flags와 compiler prerequisite가 public
distribution contract가 된다.

## Q6 LSP

현재 compiler는 parser recovery, multiple diagnostics, incremental project state, cancellation과
document overlay protocol이 없다. `SourceMap`은 in-memory text를 받을 수 있으므로 JSON consumer
prototype은 가능하지만 full LSP를 v0.7 blocker로 두면 P158 범위를 크게 넘어선다.

권장 경계:

- P158 acceptance에는 JSON diagnostic consumer fixture만 포함한다.
- stdio LSP server, incremental sync, hover/completion과 editor packaging은 v0.7 blocker에서
  제외한다.
- P158 뒤 diagnostic latency, schema stability와 maintenance owner가 확인될 때 별도 milestone
  slice로 재평가한다.

호환성 영향: 없음. v0.7은 protocol endpoint를 약속하지 않는다.

## Decision Summary

| 질문 | feasibility | 추천 | 승인 뒤 첫 구현 |
| --- | --- | --- | --- |
| Q1 | feasible | lossless token/trivia formatter | P155 |
| Q2 | feasible, compiler/backend work required | dedicated test model | P156 spec |
| Q3 | feasible, project graph refactor required | manifest-relative path dependencies | P157 |
| Q4 | feasible, error plumbing refactor required | versioned JSONL | P158 |
| Q5 | feasible on native runners | two archives, checksums, installer | P159 |
| Q6 | prototype feasible, full server premature | not a v0.7 blocker | post-P158 review |

## P154 Closure Gate

- [x] formatter trivia/comment loss inventory
- [x] test runner and entrypoint gap inventory
- [x] manifest/package graph dependency inventory
- [x] structured diagnostic schema feasibility
- [x] artifact/install/platform feasibility
- [x] LSP blocker assessment
- [x] Q1-Q6 recommendation approval

P154는 2026-07-15 승인으로 닫혔다. P155 formatter는 승인된 Q1 contract를 구현한다.
