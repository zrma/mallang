# Open Questions: v0.7 Tooling and Platforms

상태: recommendations proposed; approval pending

## Q1. Formatter architecture

추천: current AST를 그대로 pretty-print하지 않고 comments/trivia를 보존하는 token/CST
formatting layer를 먼저 설계한다. `mlg fmt <input>`은 write mode,
`mlg fmt --check <input>`은 no-write verification이며 output은 deterministic and
idempotent여야 한다.

결정 이유: current lexer/parser는 comments를 AST에 보존하지 않으므로 AST-only formatter는
source information을 삭제한다.

## Q2. Test model

추천: project `tests/` 아래 test source를 deterministic path order로 discovery하고, compiler-owned
test declaration/assertion contract를 별도 P156 spec에서 확정한다. Ordinary `func main()` 파일을
test로 간주하거나 stdout snapshot만으로 성공을 판정하지 않는다.

## Q3. Dependency boundary

추천: `mallang.toml`의 repository-relative local path dependency만 v0.7에 추가한다. Package
identity는 project name과 package path의 조합이며 dependency cycle은 project graph에서
거부한다. Registry, network fetch, semantic version solving과 lockfile은 제외한다.

## Q4. Machine-readable diagnostics

추천: existing human diagnostics를 유지하고 opt-in versioned JSON Lines mode를 추가한다.
각 record는 schema version, stage, message, source path와 byte/line/column span을 가진다.
Human text를 JSON field 하나에 감싸는 방식은 채택하지 않는다.

## Q5. Supported artifacts

추천: macOS arm64와 Linux x86_64의 `mlg` executable archive, checksum과 install script를
release artifact contract로 둔다. Artifact는 clean environment에서 version/help, project
check/build/run/test와 representative native program을 검증한다.

## Q6. LSP scope

추천: P158에서 JSON diagnostic consumer prototype까지만 필수로 하고 basic LSP는 feasibility와
maintenance cost가 확인될 때 별도 slice로 진행한다. v0.7 release blocker로 두지 않는다.

## Approval boundary

Q1-Q6은 compatibility와 tool UX를 고정하므로 P155 implementation 전에 사용자 승인이
필요하다. 승인 전에는 feasibility inventory와 fixture design만 허용한다.
