# Open Questions: v0.7 Tooling and Platforms

상태: Q1-Q6 recommendations approved on 2026-07-15

근거 문서: `feasibility.md`

## Q1. Formatter architecture

추천: current AST를 그대로 pretty-print하지 않고 comments/trivia를 보존하는 lossless
token/trivia formatting layer를 먼저 설계한다. Canonical style은 4-space indent, LF,
final newline, 연속 blank line 최대 1개로 고정하고 P155에서는 line-width wrapping을
제외한다. `mlg fmt <input>`은 write mode,
`mlg fmt --check <input>`은 no-write verification이며 output은 deterministic and
idempotent여야 한다.

결정 이유: current lexer/parser는 comments를 AST에 보존하지 않으므로 AST-only formatter는
source information을 삭제한다.

## Q2. Test model

추천: project `tests/` 아래 test source를 deterministic path order로 discovery하고, compiler-owned
test declaration/assertion contract를 별도 P156 spec에서 확정한다. Ordinary `func main()` 파일을
test로 간주하거나 stdout snapshot만으로 성공을 판정하지 않는다.

## Q3. Dependency boundary

추천: `mallang.toml`에 declaring manifest 기준 relative local path dependency만 v0.7에
추가한다. Dependency key는 target project name과 같아야 하며 alias는 지원하지 않는다.
Package identity는 project name과 package path의 조합이며 dependency cycle은 project
graph에서 거부한다. Library project와 dependency는 `src/main.mlg` 없이 check/test할 수 있고
build/run만 executable entrypoint를 요구한다. Registry, network fetch, semantic version
solving과 lockfile은 제외한다.

## Q4. Machine-readable diagnostics

추천: existing human diagnostics를 유지하고 opt-in versioned JSON Lines mode를 추가한다.
각 record는 `mallang.diagnostic.v1`, severity, stable stage, message, source path와
UTF-8 byte/1-based line-column span을 가진다. Record는 stderr에 한 줄씩 출력한다.
Human text를 JSON field 하나에 감싸는 방식은 채택하지 않는다.

## Q5. Supported artifacts

추천: macOS arm64와 Linux x86_64의 target-named `mlg` executable archive, `SHA256SUMS`와
install script를 release artifact contract로 둔다. Default install prefix는
`<home>/.local/bin`, update는 같은 installer의 explicit version 선택으로 시작한다. Artifact는
clean environment에서 compiler prerequisite를 확인하고 version/help, project
check/build/run/test와 representative native program을 검증한다.

## Q6. LSP scope

추천: P158에서 JSON diagnostic consumer prototype까지만 필수로 하고 basic LSP는 feasibility와
maintenance cost가 확인될 때 별도 slice로 진행한다. v0.7 release blocker로 두지 않는다.

## Approval Decision

Q1-Q6 추천안은 2026-07-15에 전체 승인됐다. P155부터 각 public contract를 이 문서의
경계대로 구현하며, P156의 exact test/assert syntax처럼 아직 고정하지 않은 세부 문법은
해당 milestone에서 별도 decision gate를 거친다.
