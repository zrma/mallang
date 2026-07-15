# P156 Project Test Workflow Decision

상태: recommendation ready; P156 Q1-Q6 approval pending

## Goal

`mlg test`를 ordinary `func main()` 실행이나 stdout snapshot convention이 아닌
compiler-owned workflow로 만든다. Production package/link/ownership/IR/backend 경로를
재사용하면서 test discovery, assertion failure와 process exit를 deterministic contract로
고정한다.

## Current Evidence

- `Project`는 `src/`와 exact `src/main.mlg`를 요구하며 source file을 path order로 찾는다.
- Parser/AST는 top-level type/function만 소유하고 semantic checker는 exact `func main()`을
  요구한다.
- Package graph는 source path를 `src/` 기준 package identity로 매핑하고 linker가
  package-private/public access와 internal symbol을 결정한다.
- Native backend는 하나의 C `main`을 생성하며 runtime failure는 no-unwind fatal path다.
- Cleanup은 normal return과 source control flow에 대해 검증돼 있지만 recoverable assertion
  abort 또는 test-to-test unwind contract는 없다.

이 구조에서는 한 process 안에서 assertion 실패를 복구하는 runner보다 테스트별 child
process가 현재 안전성 경계를 더 직접적으로 재사용한다.

## Q1. Declaration And Assertion Syntax

추천 syntax:

```go
test AddsValues() {
    assert(add(20, 22) == 42)
}
```

- `test`는 top-level declaration position에서만 인식하는 contextual keyword다.
- Test declaration은 identifier name, empty `()`, unit body만 가진다. `pub`, receiver,
  parameter, type parameter와 return type은 허용하지 않는다.
- `assert(expr)`는 test body의 unqualified standalone statement position에서만 compiler가
  인식하며 exact one `bool` argument를 요구한다.
- 이 contextual recognition은 test declaration 안의 nested block과 function literal까지
  이어진다. Test file의 ordinary helper `func` body는 test declaration 문맥이 아니므로
  `assert(...)`를 ordinary function call로 해석한다.
- `test`와 `assert`를 lexer keyword/global built-in으로 예약하지 않는다. Existing source의
  `func test()` 또는 `func assert(...)`는 계속 유효하다.
- Test name은 별도 package-local test namespace를 사용한다. Ordinary function/type name과
  같아도 되지만 같은 package의 duplicate test name은 거부한다.

이 선택은 attribute syntax를 새로 만들지 않고 Go-like declaration shape를 유지하며,
global identifier compatibility를 깨지 않는다. String test name과 naming-convention-only
`func TestX()`는 채택하지 않는다.

## Q2. Discovery And Package Boundary

- Optional `<project-root>/tests/`를 test root로 사용하고 `.mlg`를 recursive path order로
  찾는다.
- `tests/main_test.mlg`는 project root package, `tests/stats/*.mlg`는 source의 `stats`
  package에 대응한다. Test file의 `package` declaration은 directory와 일치해야 한다.
- Test file은 같은 package의 private production declaration에 접근할 수 있다.
- Package-private helper type/function은 test file에 둘 수 있지만 `pub` declaration은
  production API와 분리하기 위해 거부한다.
- Standalone `.mlg` test input과 external `<package>_test` package는 P156 범위 밖이다.
- Test file이 없는 project는 성공하고 `0 passed; 0 failed`를 보고한다.
- `src/main.mlg` 없는 library project test는 P157의 approved library-project discovery와
  함께 연다. P156은 현재 executable project contract를 유지한다.

## Q3. Native Execution And Isolation

- Source와 모든 test/helper declaration은 execution 전에 한 번 parse/link/semantic
  preflight를 통과해야 한다. Compile failure가 있으면 어떤 test도 실행하지 않는다.
- 선택된 각 test를 별도 synthetic native executable로 build하고 deterministic serial
  order로 실행한다.
- Application `main`은 preflight에서 검사하지만 test child의 entrypoint로 실행하지 않는다.
  Test body가 synthetic `main`이 되고 production/test helper code는 같은 compiler path를
  사용한다.
- Failed `assert`는 test ID와 source location을 포함하는 test-only fatal diagnostic을 내고
  해당 child만 non-zero로 종료한다. Parent `mlg test`는 다음 test를 계속 실행한다.
- One-binary recoverable runner, parallel execution과 in-process unwind는 P156에서 제외한다.

Per-test compilation cost는 수용한다. 현재 cleanup-aware recoverable assertion path를 새로
만드는 것보다 process isolation이 failure containment와 native behavior를 명확히 검증한다.

## Q4. Identity, Ordering And Filtering

- Stable test ID는 root package의 `<project>::<TestName>`, nested package의
  `<project>/<package>::<TestName>` 형식이다.
- Default order는 test file의 project-relative path, 그 안의 declaration source order다.
- P156 filter는 `mlg test <input> --exact <test-id>` 하나만 지원한다.
- Unknown exact ID는 silent zero-test success가 아니라 non-zero CLI diagnostic이다.
- Substring, glob, regex, tag와 ignored test는 범위 밖이다.

## Q5. Output Contract

Status와 summary는 stdout에 쓴다.

```text
test hello::AddsValues ... ok
test hello/stats::RejectsEmpty ... FAILED
test result: FAILED. 1 passed; 1 failed
```

- Passed child output은 기본적으로 capture하고 표시하지 않는다.
- Failed child의 stdout은 stdout, stderr와 normalized assertion diagnostic은 stderr로
  replay한다. 각 channel 안의 순서는 보존하지만 서로 다른 channel 사이의 출력 순서는
  contract로 두지 않는다.
- Compile/preflight failure는 test status line 없이 기존 compiler diagnostic contract를
  사용한다.
- Test result summary spelling, capitalization과 count order를 release CLI smoke로 고정한다.

## Q6. Exit And Safety Contract

- All selected tests가 성공하거나 test set이 비어 있으면 exit `0`이다.
- Compile failure, unknown filter, assertion/runtime failure 또는 child signal은 non-zero다.
- `print` output은 assertion success로 해석하지 않는다.
- Successful child는 normal cleanup과 zero-allocation accounting을 통과해야 한다.
- Assertion failure는 no-unwind child termination이며 parent process와 다음 test의 ownership
  state를 공유하지 않는다.
- Copy/non-Copy values, closure capture, recursive ADT, `Map`과 standard I/O를 포함한
  representative tests를 strict C와 ASan/UBSan acceptance에 연결한다.

## Implementation Blueprint

승인 뒤에는 다음 경계로 구현한다.

1. `Program.tests`와 `TestDecl`, `StmtKind::Assert`를 추가한다. Lexer keyword는 늘리지 않고
   parser가 top-level `test` shape와 test declaration body에서만 contextual syntax를
   인식한다. Formatter는 AST span hint를 통해 declaration/block line break를 보존한다.
2. Production source 뒤에 deterministic test path order로 하나의 `SourceMap`을 구성한다.
   Package identity는 `src/`와 `tests/` root를 같은 package path 규칙으로 매핑하고 test
   source의 `pub` type/function을 package stage에서 거부한다.
3. Package graph와 linker가 test body를 같은-package context로 연결한다. Specializer는 모든
   test body의 generic demand를 수집하고 semantic preflight는 production function, test
   helper와 모든 test/assert를 실행 전에 한 번 검사한다.
4. Checked program에서 selected test body를 synthetic `main`으로 낮추고 application `main`을
   해당 child IR에서 교체한다. `IrStmtKind::Assert`는 condition full-expression evaluation 뒤
   false일 때 no-unwind runtime failure marker를 기록하고 종료한다.
5. Parent runner는 marker의 `SourceId`/offset을 보유한 `SourceMap`으로 해석해 project-relative
   `path:line:column` diagnostic으로 바꾼다. Internal marker와 absolute project path는 public
   CLI output에 노출하지 않는다.
6. `mlg test`는 preflight 결과에서 stable test inventory를 만들고 ordinal artifact path로
   test별 C/native child를 build한다. Child output을 capture해 pass에서는 버리고 fail에서
   channel별 replay한 뒤 deterministic summary와 exit status를 집계한다.

Implementation slice는 syntax/formatter, source/package/link, semantic/specialization,
IR/backend, CLI runner, native safety/docs 순서로 닫는다. 각 slice는 focused regression 뒤
canonical `scripts/check.sh`를 유지하고 마지막 slice에서 debug/release CLI와 sanitizer를
함께 검증한다.

## Acceptance Matrix

- [x] optional `tests/` recursive deterministic discovery API
- [ ] contextual test/assert parser and formatter regression
- [ ] source/test package mapping, private access와 test-public rejection
- [ ] duplicate test ID, invalid declaration shape와 assert type diagnostics
- [ ] whole-suite preflight and per-test synthetic native build
- [ ] deterministic default order and exact filter
- [ ] pass/fail/output/exit aggregation and child signal handling
- [ ] ownership cleanup, allocation accounting, strict C and sanitizer native tests
- [ ] debug/release CLI smoke and README/SPEC/roadmap synchronization

## Approval Boundary

Q1-Q6은 새 source declaration과 public CLI/runtime behavior를 고정하므로 구현 전에 사용자
승인이 필요하다. 전체 추천안을 채택할 경우 `P156 Q1-Q6 추천안 승인`으로 승인할 수 있다.
