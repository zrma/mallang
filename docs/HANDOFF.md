# Mallang Handoff

## 현재 상태

- 언어 이름: Mallang
- 소스 확장자: `.mlg`
- CLI: `mlg`
- 공개 릴리스: `v0.1.0`부터 stable `v1.1.0`까지; macOS arm64/Linux x86_64 binary 포함
- 현재 구현: token model, hand-written lexer, AST, parser, semantic checker, entrypoint `func main()` signature checks, ownership-lite move/borrow checks, borrowed non-copy parameter escape rejection, same-call nested-field-aware borrow conflict checks, built-in value name collision checks, top-level type/function declaration name conflict checks, nested-block and arm-local shadowing with same-block redeclaration rejection, string equality without moves, guarded integer division/remainder, checked integer arithmetic, semantic printability checks, statement-only `print` semantic checks, `bool` operators with native short-circuit smoke, `|>` pipeline call sugar, statement/expression `if`, condition-only `for` loops, conditionless `for` loops, `for init; condition; post` loops, array/slice `for i, value := range values { ... }`, blank identifiers and one-variable forms in range loops, explicitly deferred mutable/by-reference range value syntax, legacy borrow alias rejection regressions, fixed-size array `values[i]` indexing for `Copy` elements with compile-time literal and native runtime bounds checks, borrowed indexing expressions for read-only non-Copy element inspection, fixed-size array `values[i] = expr` assignment for mutable `Copy` and non-copy element arrays including `for` clause post targets, fixed-size array `len(values)`, source-level owned slice type syntax `[]T`, slice literals `[]T{...}` with allocation-size/runtime allocation failure guards, `len(slice)`, Copy-only `slice[i]` value access, consuming built-in `append(slice, item) -> []T` with native realloc growth, same-field append reassignment for direct and stable indexed owned slice field paths, indexed field append-take source lowering, field-take append sources and general owned value position takes for owned slice fields, slice range with Copy value iteration and index-only non-Copy iteration, local-rooted slice field/index len/index/range/borrow reads, slice element borrow arguments for local-rooted owned slices, slice element assignment for local-rooted mutable owned slices, indexed field assignment for array/slice elements, struct cleanup for owned slice fields, internal owned slice `Type::Slice` / C `{data,len,cap}` shell and cleanup classification, internal cleanup type `mlg_drop_*` helper emission shell, explicit internal `IrStmtKind::Drop` backend lowering, straight-line cleanup param/local drop insertion before tail/return/reassignment, branch-local cleanup drop insertion for `if`/`match` statement bodies, outer cleanup root branch move normalization for `if`/`match` statements, expression-form `if`/`match` branch cleanup normalization, loop body-local cleanup drop insertion for `for`/`range` tail and `break`/`continue` paths, `for` init cleanup trailer lowering, `break`/`continue`, `else if` sugar, branch-aware return-completeness analysis, `type Name struct` declarations, named struct literals, recursive struct value-type rejection, nested field access, nested mutable field assignment, nested field-level borrow arguments, fixed-size array element borrow arguments, fixed-size array element method receivers, con/mut struct receiver methods, generic type refs, fixed-size array type refs and fixed-size array literals type-checked, fixed-size arrays as move-only values, fixed-size array typed IR/C struct-wrapper layout, `for`/`range` body C block lowering for shadowed locals, `Option`/`Result` constructor type checking, exhaustive expression/statement `match` checking, statement-form `match` block arms, non-local `match` scrutinee temp codegen, `if` expression branch prelude temp codegen, `match` expression arm prelude temp codegen, `for` clause condition/post prelude codegen with post-preserving `continue`, tagged ADT typed IR/backend layout, printable `Option`/`Result` native output, printable struct native output, typed IR, backend public API boundary with C implementation, name helper, type emitter, statement emitter, expression emitter, shared utility, unit test modules, centralized runtime error helper, native runtime failure stderr smoke coverage, and C backend IR invariant regression coverage, first native subset C backend, hidden-reference C ABI for `con`/`mut` parameters, caller-visible `mut` parameter mutation, `mlg check`, `mlg ir`, `mlg build`, `mlg run`, `mlg --version`, `mlg --help`, CLI error stream smoke, checked-in example smoke coverage guard, generated C sanitizer smoke for cleanup-heavy examples, full generated C warning-clean gate, deep generated C sanitizer sweep command, v0 release-candidate audit, `Option`/`Result` surface spec
- v0.2 기반: `SourceId`를 token/AST/IR span에 전파하고 여러 파일을 구분하는
  `SourceMap`과 file/line/column CLI diagnostic을 추가했다. `parse_sources`는 여러
  파일의 declaration을 하나의 semantic/backend compilation unit으로 합치면서
  원본 source span을 유지한다. `check_sources`, `lower_sources`,
  `generate_c_sources`는 같은 source 집합을 semantic, IR, C backend까지 전달하며
  `load_source_files`는 명시적 file 목록을 caller 순서대로 읽는다. 기존 single-file
  CLI도 이 loader와 pipeline을 사용한다. 승인된 project model에 따라
  `mallang.toml`의 project name을 읽고 directory의 가장 가까운 manifest,
  `src/main.mlg`, 재귀적 `.mlg` source 목록을 deterministic order로 찾는 project
  discovery API를 추가했다. Parser는 `package`, `import`, `pub`을 지원하고 file별
  package/import metadata와 top-level declaration visibility를 AST에 보존한다. Project
  source에서 directory package와
  declaration table, import edge, dependency-first build order를 만들고 unresolved import,
  qualifier 충돌, package mismatch, 모든 import cycle을 거부하는 package graph API를
  추가했다. Qualified function/type/struct literal을 package graph에서 충돌 없는 내부
  symbol로 연결하고 imported function/type/method의 `pub`과 public API의 private type
  노출을 검사한다. Linked project는 기존 ownership checker, typed IR, C backend를
  그대로 사용한다. Directory 또는 `mallang.toml` 입력을 project-aware `mlg check`,
  `mlg build`, `mlg run`으로 연결했으며 direct `.mlg` standalone 동작은 유지한다.
  두 package의 function/struct/method native smoke, import cycle 위치 diagnostic,
  project generated C warning-clean gate가 v0.2 acceptance를 검증한다.
- v0.3.0 released: 승인된 `func(T) U`, `func mut(T) U` function
  type과 plain/mutable function literal을 AST/parser에 추가했다. Function type은
  return type을 필수로 하며 no-value signature는 `unit`을 쓴다. Semantic checker는 named function을
  fresh move-only value로 해석하고 function parameter/return/local binding, indirect
  call argument mode, mutable callable의 exclusive access를 검사한다. C backend에는
  typed call/environment/drop pointer를 가진 callable value layout과 cleanup helper
  shell이 있다. Typed IR의 named function value/indirect call과 environment-free C
  thunk를 연결했으며 higher-order parameter/return과 반복 호출이 native로 동작한다.
  Return expression은 cleanup 전에 내부 temporary로 평가되어 callable parameter가
  조기에 drop되지 않는다. Plain function literal은 lexical free variable을 typed
  capture metadata로 보존하며 Copy capture는 복사, non-Copy capture는 생성 시 move로
  검사한다. Borrowed non-Copy capture, active range source capture, plain capture
  mutation/move-out은 거부한다. Typed IR은 closure definition과 capture value를
  보존하고 C backend는 capture environment를 할당해 call body에 연결한다. Callable
  drop thunk는 nested cleanup value를 정리한 뒤 environment를 해제한다. Escaping
  closure의 Copy/slice capture와 반복 호출이 strict C 및 sanitizer gate를 통과한다.
  Mutable closure는 대입, `mut` argument, mutable callable과 `mut` receiver 사용을
  변경 캡처로 분류하고 mutable source binding을 요구한다. `func mut` call effect와
  변경 가능성은 typed IR에 보존되며 environment field mutation, Copy 원본 격리,
  owned slice 상태 유지, nested callable cleanup이 native로 동작한다. Nested function
  literal은 lexical free variable을 바깥 closure까지 전파하고 생성 시 각 environment로
  다시 copy/move한다. Copy와 invocation-local owned value는 중첩 환경에서 안전하게
  사용할 수 있고, 반복 호출되는 바깥 환경의 borrowed non-Copy capture를 다시
  이동하는 경우는 거부한다. Project linker는 unqualified package-local function과
  imported `pkg.Function`을 value position에서 internal symbol로 연결한다. Public
  function type parameter/return, cross-package higher-order call, named function return과
  closure return이 project native warning/sanitizer gate를 통과한다.
- v0.4.0 released: generic struct/function declaration과 explicit type argument를
  project-wide demand-driven concrete specialization pass에 연결했다. 같은
  declaration/type argument key는 재사용하고 deterministic internal symbol을 만들며,
  잘못된 arity와 type argument가 계속 커지는 specialization cycle을 source
  diagnostic으로 거부한다. Concrete struct/function/function value는 기존 semantic,
  ownership, typed IR, C backend를 재사용하며 `examples/generics.mlg`의 Copy/string/slice
  specialization이 native, strict C, ASan/UBSan gate를 통과한다. 사용되지 않은 generic
  declaration도 non-Copy, non-printable symbolic type으로 검사하며 internal sentinel
  이름은 source type parameter로 복원해 진단한다. Generic receiver는 declaration type
  parameter를 그대로 binding하고 concrete struct마다 method를 생성한다. `mut`
  receiver의 non-Copy field 교체도 native cleanup gate를 통과한다. Public package
  generic struct/function/receiver와 nested imported type argument는 visibility-aware
  internal symbol로 연결되고 project native warning/sanitizer gate를 통과한다. Enum은
  generic/non-generic constructor를 concrete AST로 정규화하고 demand-driven
  specialization한다. Concrete variant/payload type, arity, recursive value type와
  cross-package visibility를 semantic에서 검사하며 internal specialization 이름을 source
  generic 표기로 복원한다. Specialized enum은 source/package pattern origin을 보존하고,
  expression/statement match에서 nested user enum과 `Option`/`Result` payload를 recursive
  coverage로 검사한다. Wildcard, duplicate/unreachable arm, payload mismatch와
  non-exhaustive path는 source diagnostic으로 거부한다. Typed IR은 concrete variant
  metadata, constructor payload와 recursive pattern tree를 보존한다. Cleanup이 필요한
  wildcard payload는 내부 owned binding으로 정규화하고 expression/statement arm의
  cleanup에 연결한다. C backend는 specialized enum마다 tag/payload union과 recursive
  drop helper를 생성하고 constructor 및 expression/statement nested match를 공통 pattern
  planner로 lowering한다. `examples/generic-enums.mlg`는 generic/user/built-in nested
  pattern과 slice payload cleanup을 native, warning-clean, ASan/UBSan 경로에서 검증한다.
  `examples/projects/hello`는 public generic enum의 imported constructor와
  package-qualified pattern, owned slice payload cleanup을 같은 native gate에서 검증한다.
  Invalid CLI fixture는 nested non-exhaustive path와 constructor payload mismatch가 source
  generic 표기 및 file/line/column으로 보고되는지 고정한다. Built-in `Option`/`Result`도
  user enum과 같은 semantic ADT metadata view, `VariantConstructor`/recursive `Variant` IR,
  tag/payload union, pattern planner와 cleanup backend를 사용한다. `Some`/`None`/`Ok`/`Err`
  source spelling과 native print output은 호환성을 유지하며, built-in 전용 IR node와 match
  emitter는 제거했다.
- v0.5 P138 완료: user enum declaration, constructor와 qualified pattern을 zero/one/multiple
  positional payload로 일반화했다. Constructor는 payload 개수, owned argument mode와 각
  위치의 type을 검사하고, match는 여러 binding, wildcard와 nested ADT pattern의 Cartesian
  exhaustiveness를 검사한다. Generic specialization과 cross-package linker는 모든 payload
  type과 pattern binding을 순회한다. Existing zero/single payload는 전체 회귀 테스트로
  호환성을 유지한다.
- v0.5 P139 완료: concrete struct/user enum dependency graph와 SCC validation을 추가했다.
  모든 recursive cycle은 user enum indirection boundary를 지나야 하며 component 내부 값을
  요구하지 않는 enum base variant가 있어야 한다. Direct/mutual struct-only recursion, mixed
  component 안의 struct-only subcycle, base 없는 enum recursion과 built-in wrapper로 감싼
  비생산적 recursion은 source diagnostic으로 거부한다. Recursive generic enum은 concrete
  specialization 뒤 검사하며 cross-package `List[T]`도 같은 경로를 사용한다. Accepted recursive
  enum 이름은 checked semantic metadata에 보존한다.
- v0.5 P140 완료: typed IR enum variant, constructor와 recursive match pattern을 positional
  payload list로 일반화했다. `IrEnumStorage::Inline`/`Owned` metadata가 non-recursive enum과
  compiler-owned recursive enum을 구분하고 constructor payload의 source evaluation order를
  보존한다. Match lowering은 모든 payload binding과 owned wildcard를 순회해 arm cleanup에
  연결하며 recursive generic enum의 constructor/pattern도 같은 IR로 내려간다. Recursive
  coverage는 유한한 opaque frontier를 사용하고 그 frontier 뒤의 구체 패턴을 wildcard
  coverage와 비교해 non-exhaustive 및 unreachable 진단을 유지한다.
- v0.5 P141 완료: C backend는 inline multi-payload variant를 positional payload struct가
  포함된 tagged union으로 생성하고, recursive enum을 compiler-owned node pointer handle로
  표현한다. Constructor는 payload를 left-to-right temporary에 평가한 뒤 inline storage 또는
  allocation-guarded node로 이동한다. Consuming match는 active payload 전체를 arm-local로
  이동하고 recursive storage shell을 한 번 해제하며, nested cleanup binding은 기존 IR drop
  경로를 사용한다. Drop helper prototype을 먼저 생성해 direct/mutual recursion을 허용하고,
  active payload를 재귀적으로 정리한 뒤 node를 해제한다. Null handle과 invalid tag는 stable
  runtime diagnostic으로 중단한다. `examples/recursive-enums.mlg`는 generic `List[int]`,
  `List[[]int]`와 non-recursive multi-payload enum을 native, strict C와 ASan/UBSan 경로에서
  검증한다.
- v0.5 P142 완료: cleanup-valued computed expression을 source syntax 변경 없이 typed IR
  `FullExprTemporary`로 모델링했다. Discarded expression, `len`/index, `con`/`mut` call
  argument와 `if`/`for` condition은 마지막 사용 뒤 정확히 한 번 정리되며, logical
  right-hand side는 short-circuit branch 안에서만 생성된다. Computed range source는 loop
  전체를 소유하고 normal exit, `break`, enclosing return에서 정리되며 `continue`에서는
  유지된다. Inline slice len/index/range 제한을 제거했고 bounds failure는 기존 fatal
  no-unwind 계약을 유지한다. `examples/full-expression-cleanup.mlg`와 500개 unit test,
  strict generated C, native output 및 60-program deep ASan/UBSan sweep이 이 계약을 검증한다.
- v0.5 P143 완료: static literal과 future heap-owned buffer를 같은 immutable move-only
  `string` value로 표현하는 tagged length-aware C runtime을 추가했다. Equality와 print는
  storage kind와 무관하게 byte length/content를 읽고, drop은 static storage를 해제하지 않으며
  owned buffer만 normal flow에서 한 번 해제한다. String parameter/return/local/field/enum payload와
  closure capture는 공통 cleanup 경로를 사용한다. Cleanup overwrite는 RHS를 먼저 평가하고 target
  place를 한 번만 계산한 뒤 old value를 drop/store하는 typed IR operation으로 정규화했다. 따라서
  side-effecting indexed target, mutable borrowed parameter와 mutable closure capture도 external owner를
  유지한다. Malformed storage/data와 allocation overflow/failure는 fatal no-unwind contract를 따른다.
  `examples/string-runtime.mlg`, 504개 unit test, strict generated C/native harness와 61-program deep
  ASan/UBSan sweep이 이 계약을 검증한다.
- v0.5 P144 완료: `con`/`mut`를 direct call-scoped argument mode로 고정하고 local/return 등
  expression position의 first-class borrow와 `con`/`mut` range binding에 전용 reserved diagnostic을
  추가했다. Borrowed non-Copy move/return/store/owned-argument/capture, use-after-move와 same-call
  overlap을 CLI fixture matrix로 고정했다. Non-Copy range는 index-only traversal 뒤
  `con users[i]`/`mut users[i]`로 읽고 변경하며, active range source ownership은 loop 뒤에도
  유지된다. `SPEC.md`는 move, overwrite, return, branch merge와 loop-persistent ownership을 같은
  normative v1 contract로 설명한다. `examples/borrow-range-contract.mlg`, 505개 unit test, strict
  generated C/native output와 62-program deep ASan/UBSan sweep이 이 경계를 검증한다.
- v0.5 P145 완료, v0.5.0 released: slice buffer, closure environment, recursive enum node와
  owned string buffer의 raw allocation/free를 공통 generated C runtime helper로 통합했다. New
  allocation lifetime과 null-buffer first growth는 live count를 증가시키고 existing realloc
  growth는 유지하며, non-null deallocation은 정확히 한 번 감소시킨다. Internal test macro는
  source/API 변경 없이 N번째 allocation attempt를 실패시키고 각 site의 stable fatal diagnostic을
  유지한다. `examples/allocation-accounting.mlg`는 slice/realloc, closure, recursive enum, return
  branch, loop, aggregate overwrite를 실행하고 normal `main` 뒤 live count 0을 검증한다. Owned
  string allocation도 같은 harness에서 별도로 계수한다. 506개 unit test, strict C/native
  accounting/failure-injection harness와 63-program deep ASan/UBSan sweep이 v0.5 memory runtime
  완료 조건을 검증한다.
- v0.6 P147-P148 완료: compiler-owned `std/...` registry가 project와 standalone import를
  같은 package/linker 경로로 resolve하고 exact public signature, explicit generic
  specialization, opaque `Map[K,V]`와 typed intrinsic call/function value identity를 보존한다.
  `std/strings`의 byte/scalar count, contains/find, split/join, int/bool format/strict parse를
  demand-driven C runtime과 callable thunk로 연결했다. String은 valid UTF-8 invariant를
  검사하고 search offset은 byte 단위, empty separator split은 Unicode scalar 단위다.
  `errors.Kind`는 payload 없는 Copy enum이며 `errors.Error`는 owned message를 가진 cleanup
  value다. Parse failure는 `InvalidData`로 반환하고 malformed compiler-owned string은 fatal
  invariant failure로 유지한다. Owned string/slice/error 결과는 공통 allocation accounting,
  normal cleanup과 deterministic failure injection을 사용한다. `examples/standard-strings.mlg`,
  edge fixture, 523개 unit test, 64-program generated C warning-clean/deep ASan/UBSan gate와
  모든 standard-string allocation 지점 failure sweep이 P148 완료 조건을 검증한다.
- v0.6 P149 완료: generated C `main`이 필요할 때만 `argc`/`argv` process ABI를 사용하고,
  `std/os.args`, `env`, `exit`와 `std/io.readStdin`, `writeStdout`, `writeStderr`를
  demand-driven runtime으로 연결한다. Arguments와 environment는 UTF-8을 검증하고 stdin은
  embedded NUL을 보존하며, missing env와 platform read/write/flush failure는
  `Result`/`errors.Error`로 반환한다. `mlg run --`은 argument와 numeric exit status를
  direct binary와 동일하게 전달한다. `examples/process-io.mlg`, process edge fixture,
  strict C, zero-allocation accounting, deterministic failure injection과 normal/error
  ASan/UBSan harness, 전체 524개 unit test와 65-program generated C sweep이 P149 완료
  조건을 검증한다.
- v0.6 P150 완료: `std/fs.readText`와 `writeText`를 demand-driven runtime과 function-value
  thunk로 연결했다. File path는 embedded NUL을 platform 호출 전에 `InvalidInput`으로
  거부하고, read는 valid UTF-8 owned string만 반환하면서 content의 NUL은 보존한다.
  Write는 create-or-overwrite exact-byte semantics이며 short write와 close failure를 성공으로
  숨기지 않는다. NotFound/PermissionDenied/InvalidData mapping, 4 KiB 초과 read growth,
  normal/error strict C, zero-allocation accounting, failure injection, ASan/UBSan harness,
  전체 525개 unit test와 66-program generated C sweep이 P150 완료 조건을 검증한다.
- v0.6 P151 완료: opaque specialized `Map[K,V]`를 compiler-owned bucket/entry layout으로
  내리고 `int`/`bool`/UTF-8 string deterministic hash/equality와 node-chain growth를
  구현했다. `insert` replacement는 incoming key를 정리하고 old value를 반환하며,
  `remove`는 stored key를 정리한 뒤 value ownership을 이전한다. `with`/`update` callback은
  저장된 value를 호출 범위에서만 빌리고, map drop은 남은 key/value와 모든 node/bucket을
  정리한다. Direct/standard function-value 호출, Copy/non-Copy value, 24-entry growth,
  zero-allocation accounting, failure injection, strict C와 ASan/UBSan harness, 전체 526개
  unit test와 67-program generated C sweep이 P151 완료 조건을 검증한다.
- v0.6 P152 완료: `examples/projects/textstats`에 arguments로 UTF-8 input file을 읽고,
  `stats` package에서 `Map[int,int]` line-length histogram과 text summary를 만든 뒤 file 또는
  stdout에 쓰는 multi-module reference CLI를 추가했다. `main.mlg`의 5개 `Result` call은
  exhaustive match 5개, Ok/Err arm 10개와 최대 3-level nesting을 만든다. Error output/exit
  mapping helper로 branch 중복은 줄지만 `unit main` process boundary는 postfix `?`만으로
  대체되지 않으므로 v0.6 syntax는 유지한다. Stdout/output-file, usage exit 2,
  missing/invalid input, write failure, strict C, zero-allocation accounting과 ASan/UBSan
  harness가 P152 완료 조건을 검증한다.
- v0.6 P153 완료, v0.6.0 released: `docs/STANDARD_LIBRARY.md`에 exact API와 ownership/failure
  semantics를 정리하고 `SPEC.md`, README와 roadmap을 구현에 동기화했다. Optimized release
  compiler가 process/file/Map/reference CLI를 다시 build/run하며 local macOS arm64 canonical,
  strict C, zero-allocation과 ASan/UBSan gate가 통과한다. Published `main`의 Ubuntu Linux
  x86_64 GitHub Actions run에서도 같은 `scripts/check.sh`가 통과해 native platform
  acceptance를 닫았다. Version bump와 서명 tag를 포함한 GitHub source release는
  2026-07-15에 공개했다.
- v0.7 P154-P155 완료: tooling/platform Q1-Q6 추천안을 승인된 contract로 고정하고,
  parser validation 뒤 raw token span과 `//` trivia를 보존하는 canonical formatter를
  추가했다. `mlg fmt`와 no-write `mlg fmt --check`는 direct `.mlg` 및 deterministic
  project source order를 지원한다. 4-space/LF/final-newline/max-one-blank-line style,
  token/comment parity, checked-in examples idempotence, project parse failure 전 파일
  no-write를 unit/debug/release smoke로 검증한다.
- v0.7 P156 완료: optional `tests/`가 `src/` package layout을 mirror하고 contextual
  `test Name()`/`assert(bool)`를 same-package private access, whole-suite preflight와
  test별 synthetic native child로 연결한다. Stable ID/order, `--exact`, pass output 억제,
  failure replay/계속 실행, empty suite와 signal fallback을 고정했다. Copy/non-Copy,
  closure, recursive ADT, Map, standard I/O fixture가 zero-allocation accounting,
  strict C, ASan/UBSan 및 debug/release CLI smoke를 통과한다.
- v0.7 P157 완료: `mallang.toml`의 exact-name relative `[dependencies]`를 canonical
  dependency-first graph로 load한다. Diamond deduplication, cycle/name/source-boundary
  rejection, direct dependency import와 transitive rejection을 package stage에 연결했다.
  Dependency `src/main.mlg`/tests는 consumer에서 제외되고 public generic/recursive API는
  기존 ownership/specialization/backend를 재사용한다. Entrypoint 없는 library는
  format/check/test를 지원하며 build/run은 stable missing-entry diagnostic을 낸다.
  Multi-project app/library test generated C가 zero-allocation, strict C, ASan/UBSan과
  debug/release smoke를 통과한다.
- v0.7 P158 완료: compiler-owned error를 versioned `mallang.diagnostic.v1` 모델로
  통합하고 global `--diagnostic-format <human|json>`을 추가했다. Human과 JSON은 같은
  stage/message/source/span을 렌더링하며 UTF-8 byte offset, 1-based Unicode scalar
  location과 root/dependency project path를 고정한다. CLI/input/frontend/package/link/
  semantic/native binary matrix, formatter multi-record, failed test assertion와 JSONL
  consumer가 debug/release smoke를 통과한다. Full LSP는 P160의 v0.8 decision gate까지
  보류한다.
- v0.7 P159 완료: macOS arm64/Linux x86_64 target-named archive,
  normalized tar/gzip metadata, exact `SHA256SUMS`와 `MIT OR Apache-2.0` payload를 추가했다.
  Explicit-version installer는 HTTPS 또는 paired offline input을 사용하고 checksum, archive
  entry set와 staged binary version을 검증한 뒤 default/explicit prefix의 `mlg`를 atomic
  replace한다. Canonical local gate는 deterministic rebuild, tamper/malformed rejection,
  reinstall과 installed project check/build/run/test를 검증한다. Published GitHub Actions에서
  두 native job과 canonical job이 통과했고 downloaded combined bundle의 두 checksum도
  일치했다.
- v0.7 P160 완료, v0.7.0 released:
  `scripts/check-v07-acceptance.sh`가 빈 work directory에 local
  library와 dependent executable project를 만들고 deterministic archive에서 clean prefix로
  설치한 `mlg`만 사용한다. Formatter no-write/idempotence, human/JSON check, isolated test,
  native build/direct run과 compiler run을 하나의 workflow로 검증하며 canonical check와 두
  supported-platform release job이 같은 script를 호출한다. Published CI run
  `29433381232`에서 Ubuntu canonical check, macOS arm64/Linux x86_64 acceptance와 combined
  checksum bundle이 모두 통과했고 downloaded 두 archive checksum도 일치했다. README와
  `SPEC.md`에 manual new project 경로를 문서화했다. v0.8 hardening Q1-Q6는 2026-07-16
  승인됐고 v0.7.0 GitHub Release는 installer, checksum과 두 native archive를 제공한다.
- v0.8 P161 완료: lexer, parser, multi-source frontend와 compiler가 모두 first-error
  `Result`로 중단하지만 CLI는 이미 `Vec<Diagnostic>` human/JSON rendering을 지원함을
  확인했다. Production invariant site는 user-reachable diagnostic, malformed IR validator,
  locally proven assertion의 세 범주로 audit한다. Property/crash corpus와 generated C repeated
  byte gate는 아직 없고, P165 representative baseline은 minimal standalone,
  cleanup-heavy standalone, local-dependency app와 standard-library reference CLI로 고정했다.
  P162는 top-level recovery/aggregation, block recovery, cap/compatibility acceptance 순서다.
- v0.8 P162 Slice A 완료: 기존 single-error API를 유지하면서 parser, frontend와 compiler에
  ordered multi-diagnostic API를 추가했고 CLI `parse/check/ir/build/run/test`가 이를 사용한다.
  Top-level recovery는 mandatory progress와 delimiter depth를 적용하고 source별 32개로
  제한한다. 두 source human/JSON parity, stable source order와 frontend error 뒤 semantic
  미진입을 unit/CLI regression으로 검증했다. Block statement recovery와 compatibility
  acceptance는 후속 Slice B/C로 분리했다.
- v0.8 P162 Slice B 완료: block parser는 `;`, current `}`, unambiguous statement keyword만
  recovery boundary로 사용한다. Named function declaration이 unclosed block 안에서 보이면
  missing `}`를 보고하고 top-level recovery로 넘긴다. Nested function literal은 block 안에
  유지하고 `func (` receiver method는 모호한 recovery target에서 제외한다. Source 전체
  32-error cap, unmatched parenthesis cascade 방지와 top-level/block 4-record CLI parity를
  회귀로 고정했다. Slice C의 duplicate suppression과 compatibility acceptance가 다음이다.
- v0.8 P162 Slice C 완료: parser는 exact `(message, span)` duplicate만 제거하고 source 내부
  span stable order를 고정한다. 40-error input은 concrete error 첫 32개만 반환하며 lexical
  error는 first-error를 유지한다. `scripts/check-parser-recovery.sh`가
  `parse/check/ir/build/run/test` human/JSON parity, non-zero, empty stdout와 single/multi API
  compatibility를 canonical acceptance로 검증한다. P162 전체가 완료됐고 P163 invariant
  defense가 다음이다.
- v0.8 P163 완료: production panic-like site를 user-reachable diagnostic, malformed internal
  IR, locally proven invariant로 재분류했다. Direct `Parser::new`가 EOF sentinel을 자체
  보장하고 pattern extraction과 receiver diagnostic의 `expect`/`unwrap`을 제거했다. Empty
  match는 semantic/IR diagnostic으로 거부한다. C backend는 출력 전 duplicate type/field/
  variant/function/parameter/capture와 invalid `main` signature를 검사하며 기존 fragment
  program과 local expression/statement invariant validator를 유지한다. Malformed source의
  frontend/package/semantic stage와 malformed IR negative matrix를 회귀로 고정했다.
- v0.8 P164 완료: stable Rust만 사용하는 deterministic hardening integration test를 추가했다.
  256개 seed의 syntax-heavy arbitrary UTF-8 lexer input은 token/error span bound와 UTF-8
  boundary를 검사한다. 네 valid token stream의 모든 token에 delete/duplicate/five-kind replace를
  적용해 parser가 bounded ordered diagnostic 또는 program을 반환하는지 검증한다. 다섯 valid
  program의 type/ownership marker를 하나씩 invalid하게 바꾸고 semantic message class를 고정했다.
  `tests/fixtures/hardening/crash-corpus/`의 최소 source 6개는 frontend/package/link/semantic/
  ownership stage diagnostic과 corpus registration completeness를 검증한다.
- v0.8 P165 완료: release-profile `mlg`로 minimal, cleanup-heavy, local dependency와
  standard-library CLI를 반복 측정해 `docs/baselines/v0.8-performance.json`에 observational
  baseline을 남겼다. `scripts/check-v08-reproducibility.sh`는 baseline schema와 네 generated C
  반복 빌드 및 기존 release archive의 byte identity를 검증한다. Native executable identity는
  host C toolchain 영향 때문에 명시적으로 제외한다.
- v0.8 P166 완료, v0.8.0 released: debug/release `mlg`가 checked-in crash corpus와 parser
  recovery를 동일한 stage-owned diagnostic으로 처리한다. `scripts/check-v08-acceptance.sh`는
  canonical check, optimized release binary와 complete generated C ASan/UBSan sweep을 구성하며
  macOS arm64/Linux x86_64 matrix와 checksum bundle이 같은 boundary를 검증한다. Performance
  threshold는 supported-platform 반복 표본 전까지 observational로 유지한다. v0.9 Q1-Q6는
  승인됐고 현재 v0.8 language surface를 v1 candidate로 동결한다.
- v0.9 P167 완료: `docs/V1_LANGUAGE_CONTRACT.md`에 source, project, type, ownership,
  standard library, CLI/diagnostic와 native distribution surface를 85개 stable rule ID로
  inventory했다. Copy/move classification, user enum과 nested match의 stale `SPEC.md` 문구를
  current implementation에 맞게 교정했다.
- v0.9 P168 완료: `docs/COMPATIBILITY.md`와 `V1-COMP-001`-`013`이 compiler/language
  version relation, v1.x source/semantic compatibility, release class, deprecation,
  soundness exception과 no-edition policy를 고정한다. P169는 총 98개 rule의 conformance
  evidence와 v0.x migration을 통합한다.
- v0.9 P169 완료: `docs/conformance/v1-rules.json`은 98개 rule을 23개 profile과
  64개 evidence item에 exact-set으로 연결하고 `scripts/check-v1-conformance.py`가 stale
  rule/path/test를 fail-closed 검사한다. `docs/MIGRATION_V1.md`와
  `scripts/check-v1-migration.sh`는 canonical `con`/`mut`, index-only non-Copy range와
  legacy bootstrap syntax rejection을 check/build/run acceptance로 고정한다.
- v0.9 P170 완료: `scripts/check-v09-dogfood.sh`는 deterministic release archive의
  clean-prefix compiler로 clean-copy `textstats`를 format/check/test/build/run 두 번 수행하고
  source/output/generated C identity, error flow, strict C, sanitizer와 allocation accounting을
  검증한다. Existing production source 두 파일의 formatter canonicalization과 UTF-8 summary
  project test 누락, generic empty-suite fixture coupling만 test/maintenance gap으로 수정했으며
  frozen surface 변경이나 unresolved release blocker는 없다.
- v0.9 P171 완료, v0.9.0 released: `scripts/check-v09-freeze.py`는 signed v0.8.0 base 이후
  compiler source 변경 0건과 documentation/conformance/dogfood/release classification을
  검증한다. `scripts/check-v09-acceptance.sh`는 98 rules/23 profiles/64 evidence completeness,
  canonical and optimized compiler, complete generated C sanitizers와 clean-install dogfood를
  macOS arm64/Linux x86_64 artifact matrix 및 checksum bundle에 연결한다.
- v0.9 P172 완료, v1.0.0-rc.1 prerelease: release tooling은 exact SemVer prerelease를
  archive/checksum/installer/version 전체에서 처리한다. `scripts/check-v1-rc-rehearsal.sh`는
  같은 prefix에서 published v0.9.0 설치, RC upgrade, explicit rollback과 RC re-upgrade를
  수행하며 매 단계 clean-copy `textstats` observable output identity를 검증한다. 두 supported
  platform의 full RC acceptance가 통과했고 unresolved stable blocker는 없다.
- v1.0.0 stable 완료: v0.9.0 이후 compiler source와 98-rule conformance map이
  변경되지 않았음을 final gate가 확인한다. Published RC에서 stable upgrade, RC rollback과
  stable re-upgrade를 같은 prefix와 clean-copy `textstats`로 검증하며 macOS arm64/Linux
  x86_64 archive, combined checksum, signed tag와 public stable release를 제공한다.
- v1.1.0 released: `std/fs.forEachLine[C,S]`가 source-visible handle 없이
  bounded-memory UTF-8 line processing을 제공한다. Published v1.0.0에서 upgrade,
  rollback, re-upgrade하며 기존 `textstats` output identity와 새 streaming 동작을 함께
  검증했다. macOS arm64/Linux x86_64 artifact, combined checksum, signed tag와 public
  stable release를 제공한다.
- 아직 없음: first-class borrowed references, statement-spanning borrow lifetimes, general partial moves from fields beyond slice field take, full C backend, method values/interfaces/dynamic dispatch. `con expr` / `mut expr` remain call argument mode prefixes only; statement-spanning borrow syntax is explicitly deferred. Non-slice field partial moves remain explicitly deferred; owned slice field take is the only v0 field-take exception.

## 빠른 시작

```sh
scripts/check-agent-harness-interface.sh
scripts/check.sh
scripts/check-v08-acceptance.sh
scripts/check-parser-recovery.sh target/debug/mlg
scripts/check-v07-acceptance.sh
scripts/check-v09-dogfood.sh
scripts/check-v09-acceptance.sh
scripts/check-v1-rc-rehearsal.sh
scripts/check-v1-rc-acceptance.sh
scripts/check-v1-stable-rehearsal.sh
scripts/check-v1-stable-acceptance.sh
scripts/check-v1x-acceptance.sh
scripts/check-v1x-upgrade.sh
scripts/check-release-artifacts.sh
scripts/check-release-binary.sh
scripts/check-release-helpers.sh
scripts/check-generated-c-sanitizers.sh --assume-generated
scripts/verify-v0-rc.sh
scripts/finalize-and-push.sh --verify-only
VERSION=1.1.0
scripts/finalize-and-push.sh --message "chore: publish mallang ${VERSION}" --no-push
cargo run --bin mlg -- --version
cargo run --bin mlg -- --help
cargo run --bin mlg -- check examples/first.mlg
cargo run --bin mlg -- --diagnostic-format json check examples/first.mlg
cargo run --bin mlg -- test examples/projects/hello
cargo run --bin mlg -- run examples/projects/local-deps/app
cargo run --bin mlg -- run examples/function-values.mlg
cargo run --bin mlg -- run examples/closures.mlg
cargo run --bin mlg -- run examples/mutable-closures.mlg
cargo run --bin mlg -- run examples/nested-closures.mlg
cargo run --bin mlg -- run examples/full-expression-cleanup.mlg
cargo run --bin mlg -- run examples/string-runtime.mlg
cargo run --bin mlg -- run examples/borrow-range-contract.mlg
cargo run --bin mlg -- run examples/allocation-accounting.mlg
cargo run --bin mlg -- run examples/standard-strings.mlg
printf 'input' | MALLANG_P149_TEST=값 cargo run --bin mlg -- run examples/process-io.mlg -- alpha
printf 'text' > target/mallang/file-input.txt
cargo run --bin mlg -- run examples/file-io.mlg -- target/mallang/file-input.txt target/mallang/file-output.txt
cargo run --bin mlg -- check examples/projects/hello
cargo run --bin mlg -- build examples/projects/hello
cargo run --bin mlg -- run examples/projects/hello/mallang.toml
cargo run --bin mlg -- ir examples/adt.mlg
cargo run --bin mlg -- build examples/first.mlg -o target/mallang/first
target/mallang/first
cargo run --bin mlg -- build examples/if-statement.mlg -o target/mallang/if-statement
target/mallang/if-statement
cargo run --bin mlg -- build examples/for-loop.mlg -o target/mallang/for-loop
target/mallang/for-loop
cargo run --bin mlg -- build examples/loop-control.mlg -o target/mallang/loop-control
target/mallang/loop-control
cargo run --bin mlg -- build examples/for-clause.mlg -o target/mallang/for-clause
target/mallang/for-clause
cargo run --bin mlg -- build examples/for-clause-initless.mlg -o target/mallang/for-clause-initless
target/mallang/for-clause-initless
cargo run --bin mlg -- build examples/for-empty-condition.mlg -o target/mallang/for-empty-condition
target/mallang/for-empty-condition
cargo run --bin mlg -- build examples/for-clause-prelude.mlg -o target/mallang/for-clause-prelude
target/mallang/for-clause-prelude
cargo run --bin mlg -- build examples/int-division.mlg -o target/mallang/int-division
target/mallang/int-division
cargo run --bin mlg -- build examples/checked-arithmetic.mlg -o target/mallang/checked-arithmetic
target/mallang/checked-arithmetic
cargo run --bin mlg -- check examples/arrays.mlg
cargo run --bin mlg -- ir examples/arrays.mlg
cargo run --bin mlg -- build examples/arrays.mlg -o target/mallang/arrays
target/mallang/arrays
cargo run --bin mlg -- build examples/slice-append.mlg -o target/mallang/slice-append
target/mallang/slice-append
cargo run --bin mlg -- build examples/slice-range.mlg -o target/mallang/slice-range
target/mallang/slice-range
cargo run --bin mlg -- build examples/slice-element-borrow.mlg -o target/mallang/slice-element-borrow
target/mallang/slice-element-borrow
cargo run --bin mlg -- build examples/slice-element-assignment.mlg -o target/mallang/slice-element-assignment
target/mallang/slice-element-assignment
cargo run --bin mlg -- build examples/slice-field-append.mlg -o target/mallang/slice-field-append
target/mallang/slice-field-append
cargo run --bin mlg -- build examples/indexed-slice-field-append.mlg -o target/mallang/indexed-slice-field-append
target/mallang/indexed-slice-field-append
cargo run --bin mlg -- build examples/slice-field-take-append.mlg -o target/mallang/slice-field-take-append
target/mallang/slice-field-take-append
cargo run --bin mlg -- build examples/slice-field-take.mlg -o target/mallang/slice-field-take
target/mallang/slice-field-take
cargo run --bin mlg -- build examples/indexed-field-assignment.mlg -o target/mallang/indexed-field-assignment
target/mallang/indexed-field-assignment
cargo run --bin mlg -- build examples/indexed-field-read.mlg -o target/mallang/indexed-field-read
target/mallang/indexed-field-read
cargo run --bin mlg -- build examples/struct-slice-field.mlg -o target/mallang/struct-slice-field
target/mallang/struct-slice-field
cargo run --bin mlg -- build examples/slice-field-read.mlg -o target/mallang/slice-field-read
target/mallang/slice-field-read
cargo run --bin mlg -- build examples/slice-field-assignment.mlg -o target/mallang/slice-field-assignment
target/mallang/slice-field-assignment
cargo run --bin mlg -- build examples/range-blank.mlg -o target/mallang/range-blank
target/mallang/range-blank
cargo run --bin mlg -- build examples/range-index.mlg -o target/mallang/range-index
target/mallang/range-index
cargo run --bin mlg -- run examples/range-index.mlg
cargo run --bin mlg -- build examples/non-copy-array-assignment.mlg -o target/mallang/non-copy-array-assignment
target/mallang/non-copy-array-assignment
cargo run --bin mlg -- build examples/array-for-post.mlg -o target/mallang/array-for-post
target/mallang/array-for-post
cargo run --bin mlg -- build examples/string-equality.mlg -o target/mallang/string-equality
target/mallang/string-equality
cargo run --bin mlg -- build examples/logical-operators.mlg -o target/mallang/logical-operators
target/mallang/logical-operators
cargo run --bin mlg -- build examples/pipeline.mlg -o target/mallang/pipeline
target/mallang/pipeline
cargo run --bin mlg -- build examples/adt.mlg -o target/mallang/adt
target/mallang/adt
cargo run --bin mlg -- build examples/print-adt.mlg -o target/mallang/print-adt
target/mallang/print-adt
cargo run --bin mlg -- build examples/match-temp.mlg -o target/mallang/match-temp
target/mallang/match-temp
cargo run --bin mlg -- build examples/if-match-expression.mlg -o target/mallang/if-match-expression
target/mallang/if-match-expression
cargo run --bin mlg -- build examples/match-arm-prelude.mlg -o target/mallang/match-arm-prelude
target/mallang/match-arm-prelude
cargo run --bin mlg -- build examples/structs.mlg -o target/mallang/structs
target/mallang/structs
cargo run --bin mlg -- build examples/print-struct.mlg -o target/mallang/print-struct
target/mallang/print-struct
cargo run --bin mlg -- build examples/methods.mlg -o target/mallang/methods
target/mallang/methods
cargo run --bin mlg -- build examples/mut-receiver.mlg -o target/mallang/mut-receiver
target/mallang/mut-receiver
cargo run --bin mlg -- build examples/field-assignment.mlg -o target/mallang/field-assignment
target/mallang/field-assignment
cargo run --bin mlg -- build examples/field-borrow.mlg -o target/mallang/field-borrow
target/mallang/field-borrow
cargo run --bin mlg -- build examples/array-element-borrow.mlg -o target/mallang/array-element-borrow
target/mallang/array-element-borrow
cargo run --bin mlg -- build examples/array-element-methods.mlg -o target/mallang/array-element-methods
target/mallang/array-element-methods
cargo run --bin mlg -- build examples/mut-parameter-abi.mlg -o target/mallang/mut-parameter-abi
target/mallang/mut-parameter-abi
cargo run --bin mlg -- build examples/nested-fields.mlg -o target/mallang/nested-fields
target/mallang/nested-fields
cargo run --bin mlg -- build examples/return-completeness.mlg -o target/mallang/return-completeness
target/mallang/return-completeness
cargo run --bin mlg -- build examples/else-if.mlg -o target/mallang/else-if
target/mallang/else-if
cargo run --bin mlg -- build examples/match-statement.mlg -o target/mallang/match-statement
target/mallang/match-statement
```

## 주요 문서

- `docs/agent-harness.md`: 이 저장소의 canonical 하네스 구조와 Mallang overlay
- `SPEC.md`: published stable Mallang v1 language/tooling contract
- `docs/V1_ROADMAP.md`: `v0.2.0`부터 `v1.0.0`까지 아홉 개 장기 milestone과 완료 조건
- `docs/todo-v03-functions-closures/`: v0.3 function value와 owned closure decision gate
- `docs/todo-v04-generic-data-model/`: v0.4 generic enum과 static specialization decision gate
- `docs/todo-v05-ownership-runtime/`: v0.5 minimal ownership model과 transparent recursive ADT contract
- `docs/todo-v06-standard-library/`: approved v0.6 contract and completed P147-P153 acceptance evidence
- `docs/todo-v07-tooling-platforms/`: approved P154-P160 contract and P155-P160 implementation evidence
- `docs/todo-v08-compiler-hardening/`: completed P161-P166 v0.8 hardening contract
- `docs/todo-v09-language-freeze/`: approved P167-P172 v0.9 freeze contract
- `docs/todo-v11-streaming-text-io/`: released v1.1 additive streaming text I/O and acceptance
- `docs/todo-naming-conventions/`: approved role-based casing debt, lint/fix plan and 2.0 migration boundary
- `docs/SELF_HOSTING.md`: active B0-B5 self-hosting and fixed-point contract
- `docs/todo-self-hosting-bootstrap/`: closed B0 bootstrap feasibility and decisions
- `docs/todo-self-hosting-frontend/`: closed B1 frontend differential contract
- `docs/todo-self-hosting-semantics/`: closed B2 semantics and typed-IR contract
- `docs/todo-self-hosting-backend/`: active B3 Mallang C backend contract
- `docs/todo-self-hosting-loop-performance/`: B2 inner-loop performance and
  focused/fast/full gate performance and full-gate preservation contract
- `docs/releases/`: v0.1.0부터 v1.1.0까지의 release notes와 verification record
- `SECURITY.md`: stable supported version과 private vulnerability reporting policy
- `ROADMAP.md`: compiler milestone
- `docs/ROADMAP.md`: agent가 다음 작업을 고르는 운영용 roadmap
- `docs/REPO_MANIFEST.yaml`: 검증 명령과 entrypoint 선언
- `docs/ESCALATION_POLICY.md`: 사용자 호출 조건

## v1 이후 변경 원칙

1. 모든 1.x 변경은 `docs/COMPATIBILITY.md`와 기존 rule ID를 기준으로 분류한다.
2. Source-visible 변경은 backward-compatible addition이거나 명시적인 soundness/security
   exception이어야 하며, regression, migration과 release note를 함께 추가한다.
3. 다음 milestone은 구체적인 repo-owned gap과 acceptance evidence가 있을 때 별도
   `docs/todo-*` 문서로 시작한다.

현재 post-v1 작업은 B0-B5 self-hosting program이다. B0에서 Rust Stage0이 tracked
Mallang probe를 format/check/test/build/run하고 same-input generated C identity를
검증했다. B1은 frozen v1 frontend와 bounded recovery를 완성하고 155개 repository
source를 Rust Stage0, generated Stage1, strict accounting, sanitizer 네 경로에서
byte-for-byte differential 검증해 닫았다. B2는 비제네릭 선언과 resolved type
signature, primitive function body, named function value와 direct/indirect call을
Mallang으로 검사하고 field/index read까지 incremental typed IR을 Rust Stage0과
비교하며 mutable field/index assignment place도 검사한다. integrated gate는 159개
repository source와 227개 semantic fixture, 6개 IR fixture를
normal/strict/sanitizer 경로에서 검증한다. nested lexical scope, if-statement
return convergence와 if-expression branch type convergence까지 구현됐으며
non-Copy local move, nested field/index `con`/`mut` borrow와 same-call overlap도
검사한다. statement/expression `if`의 outer move state를 병합하고 condition 및
conditionless loop의 condition/body persistent move도 거부한다. three-clause의
loop-scoped init, optional condition과 direct/field/index post도 검사한다. range
source read와 Copy/index-only binding, body scope, active-source assignment 및
persistent outer move도 검사한다. method receiver의 owned move, `con` read,
`mut` mutability와 receiver/argument borrow overlap을 direct local, local-rooted
field/index 및 temporary/computed base에서 검사한다. explicit struct/array/slice
literal의 completeness, length, type과 owned element move도 검사하며 94개 Mallang
compiler project test가 이를 고정한다. P176d1b1은 call, return, assignment, nested
field/element와 if-expression branch에서 해당 literal의 expected type을 전파하며
전체 104개 Mallang compiler project test가 이를 고정한다. P176d1b2a의
`None`/`Some`/`Ok`/`Err`는 expected Option/Result context, arity, owned argument와
nested payload literal을 검사하며 전체 114개 project test가 이를 고정한다.
P176d1b2b는 known non-generic user enum의 zero/one/
multiple payload constructor를 expected enum/payload type, arity, owned mode와
left-to-right move 순서로 검사하며 전체 125개 project test가 이를 고정한다.
P176d1b2c1은 flat Option/Result expression match의 pattern type, wildcard,
duplicate/unreachable arm, exhaustiveness, arm binding scope, expected type과
outer move join을 검사하며 전체 137개 project test가 이를 고정한다. 다음 slice는
P176d1b2c2a flat statement match의 arm scope, return convergence, loop control과
outer move join을 검사하며 전체 145개 project test가 이를 고정한다.
P176d1b2c2b1은 flat non-generic user enum의 pattern type/variant, payload arity와
binding type, wildcard/duplicate/exhaustive coverage 및 scrutinee move를 검사하며
전체 158개 project test가 이를 고정한다. P176d1b2c2b2는 nested
built-in/user enum payload와 multi-payload Cartesian path, recursive sentinel을
통해 finite coverage를 검사하며 전체 170개 project test가 이를 고정한다.
P176d2a1은 capture-free plain/mutable function literal, private callable signature,
return completeness와 mutable indirect call을 검사하며 전체 180개 project test가
이를 고정한다. P176d2a2는 plain closure capture를 first-use 순서 metadata로
보존하고 Copy capture는 원본을 유지하며 non-Copy capture는 생성 시 move한다.
Moved value, borrowed non-Copy와 active range source capture를 거부하고 shadowed
binding/parameter 경계를 전체 191개 project test로 고정한다. P176d2a3에서
mutable capture의 direct assignment, `mut` argument/receiver,
immutable source rejection과 nested Copy/mutable propagation을 구현하고 recursive
initializer를 거부하며 전체 201개 project test로 고정한다. P176e1은 plain,
mutable, nested closure definition과 parameter, capture metadata, closure value를
typed IR에 보존하며 9개 IR fixture와 전체 203개 project test로 고정한다.
P176e2a는 straight-line owned parameter/local의 역순 drop, moved root 제외와
return value 선평가 temporary를 typed IR에 삽입하며 10개 IR fixture와 전체
204개 project test로 고정한다. P176e2b1은 `if` branch-local cleanup의
tail/return drop과 enclosing statement span을 11개 IR fixture와 전체 205개
project test로 고정한다. P176e2b2는 non-shadowing outer cleanup root의
keep/one-side move/both-side
move/both-side return과 nested join을 12개 IR fixture와 전체 206개 project
test로 고정한다. P176e2b3는 name과 declaration span으로 cleanup binding을
식별해 branch-local shadow의 move/drop을 동명의 outer root와 분리하고 assignment
reactivation은 원래 identity를 유지한다. Rust 582개, 13개 IR fixture와 전체
207개 Mallang project test가 이를 고정한다. P176e2c1은 direct local cleanup
assignment의 RHS를 stable temporary로 먼저 평가한 뒤 `Stmt.Overwrite`하고,
self-consuming assignment는 같은 binding identity로 재활성화한다. 14개 IR
fixture와 전체 208개 Mallang project test가 이를 고정한다. P176e2c2는
non-self-consuming field/index cleanup assignment에도 같은
RHS-before-overwrite 계약을 적용하고 aggregate base는 move하지 않는다. 15개 IR
fixture와 전체 209개 Mallang project test가 이를 고정한다. P176e2c3a는 `mut`
cleanup parameter와 mutable cleanup capture를 external root로 유지해 RHS-first
overwrite 뒤 caller/environment-owned root의 tail drop을 생략한다. 16개 IR
fixture와 전체 210개 Mallang project test가 이를 고정한다. P176e2c3b는
`append`의 field source를 move하지 않는 `Expr.SliceAppend`로 낮추고 direct/indexed
동일 field path에서는 `Stmt.FieldAssign`을 보존하되 다른 source path는 기존
RHS-first overwrite를 유지한다. 17개 IR fixture와 전체 212개 Mallang project
test가 이를 고정한다. P176e2c3c는 direct slice self-append reactivation,
field-source read와 non-Copy item move를 18개 IR fixture와 전체 214개 project
test로 고정한다. P176e2c3d는 read-only `len`을 `Expr.ArrayLen`으로 낮추고 return
value를 먼저 평가한 뒤 owned source를 drop한다. 19개 IR fixture와 전체 216개
project test가 이를 고정한다. P176e2c3e는 statement-only `print` source를
`Arg.Con` read로 낮추고 출력 뒤 owned direct/field owner를 tail drop한다. 20개 IR
fixture와 전체 218개 project test가 이를 고정한다. P176e2c3f는 struct literal
field를 선언 순서의 `Field.Value`로 낮추고 owned field source를 정확히 한 번만
소비한다. 21개 IR fixture와 전체 219개 project test가 이를 고정한다.
P176e2c3g는 fixed-array와 slice literal을 `Expr.ArrayLiteral`로 낮추고 owned
element를 정확히 한 번만 소비한다. 22개 IR fixture와 전체 220개 project test가
이를 고정한다. P176e2c3h는 `None`/`Some`/`Ok`/`Err`를 inline
`Expr.VariantConstructor`로 낮추고 owned payload를 정확히 한 번만 소비한다. 23개
IR fixture와 전체 221개 project test가 이를 고정한다. P176e2c3i는 user enum
constructor를 type graph 기반 `Inline`/`Owned` storage로 낮추고 zero/one/multiple
payload를 정확히 한 번만 소비한다. 24개 IR fixture와 전체 222개 project test가
이를 고정한다. P176e2c3j는 Copy `Option`/`Result` expression match를 명시적인
match-arm과 inline variant pattern IR로 낮춘다. P176e2c3k는 cleanup payload move,
arm-local drop, cleanup wildcard 합성 binding과 string read full-expression
temporary까지 확장한다. 26개 IR fixture와 전체 224개 project test가 이를
고정한다. P176e2c3l은 flat user-enum expression match를 graph-derived
`Inline`/`Owned` pattern storage, multiple payload binding과 cleanup wildcard로
낮춘다. 27개 IR fixture와 전체 225개 project test가 이를 고정한다. P176e2c3m은
nested built-in/user-enum pattern과 recursive owned enum을 재귀적으로
낮추고 arm-wide cleanup wildcard 번호를 유지한다. 28개 IR fixture와 전체 226개
project test가 이를 고정한다. P176e2c3n은 statement-form match block, arm-local
payload cleanup과 outer branch move compensation을 낮춘다. 29개 IR fixture와 전체
227개 project test가 이를 고정한다. P176e2c3o는 test declaration body checking,
`assert(bool)` statement와 선택된 test를 synthetic `main`으로 만드는 test typed IR을
추가한다. 29개 IR fixture와 전체 230개 project test가 이를 고정한다. P176e2c3p는
condition/conditionless `for`, `break`/`continue`, explicit body/cleanup block typed IR을
추가한다. 30개 IR fixture와 전체 231개 project test가 이를 고정한다. P176e2c3q는
three-clause `for`의 init/optional condition/post typed IR을 추가한다. 31개 IR fixture와
전체 232개 project test가 이를 고정한다. P176e2c3r은 Copy element array/slice range의
binding/element type, source, body/cleanup typed IR과 owned range source read cleanup을
추가한다. 32개 IR fixture와 전체 233개 project test가 이를 고정한다. P176e2c3s는
owned for-init exit cleanup과 loop/range body-local cleanup을 normal tail 및
`break`/`continue` 경로에 삽입한다. 33개 IR fixture와 전체 234개 project test가 이를
고정한다. P176e2c3t는 index-only non-Copy range와 indexed `con` element borrow typed
IR을 추가한다. 34개 IR fixture와 전체 235개 project test가 이를 고정한다. P176e2c3u는
method declaration을 `ReceiverType.method` IR 함수로 내리고 direct local
owned/`con`/`mut` receiver를 첫 call argument로 보존한다. 35개 IR fixture와 전체
236개 project test가 이를 고정한다. P176e2c3v는 direct field/index receiver와
temporary/computed field/index receiver를 place-aware typed IR로 내리고 computed
owner의 move를 보존한다. 36개 IR fixture와 전체 237개 project test가 이를 고정한다.
P176e2c3w는 computed array 및 projected-field `len` source를 동일한
full-expression place 경로로 낮추고 이미 move된 owner의 tail drop을 제거한다. 37개 IR
fixture와 전체 238개 project test가 이를 고정한다. P176e2c3x는 inline/projected range
temporary owner를 normal exit와 early control-flow cleanup까지 유지하고 consumed source
move를 tail cleanup에서 제외한다. 38개 IR fixture와 전체 239개 project test가 이를
고정한다. P176e2c3y는 expression-form `if`의 branch move를 explicit then/else cleanup
block으로 정규화하고 Rust oracle도 동일 schema를 사용한다. 39개 IR fixture와 전체
240개 project test가 이를 고정한다. P176e2c3z는 expression-form `match`의 outer-owner
compensation drop을 각 arm의 기존 pattern cleanup 뒤에 추가한다. 40개 IR fixture와
전체 241개 project test가 이를 고정한다. P176e3a는 demand-driven generic
struct/function과 declaration-bound receiver method를 concrete AST로 specialization한 뒤
기존 semantic/ownership/typed-IR core를 재사용한다. Named/slice/fixed-array type
argument와 cleanup generic call의 `print` full-expression temporary를 230개 semantic
fixture, 41개 IR fixture와 전체 242개 project test가 고정한다. P176e3b는 generic enum
constructor를 concrete enum identity로 rewrite하고 source generic pattern origin을 별도로
보존한다. 두 concrete specialization, owned payload cleanup과 wrong-arity rejection을 232개
semantic fixture, 42개 IR fixture와 전체 243개 project test가 고정한다. P176e3c는
사용 여부와 무관하게 모든 generic declaration을 non-Copy, non-printable symbolic type
argument로 한 번 specialization하고 semantic checker에 전달한다. Internal
symbolic/specialization 이름은 source `T`/`Box[T]` 표기로 복원하며 네 rejection fixture가
Rust oracle의 declaration order, stage, span과 message에 바이트 단위로 일치한다. 236개 semantic fixture,
42개 IR fixture와 전체 244개 project test가 이를 고정한다. P176e4a는 token과 lexical
diagnostic에 source ID를 보존하고 parser arena를 source-unit/struct/enum/function/test
group 순서로 deterministic merge한다. `parse-sources` mode의 valid/rejection source set
2개가 Rust `parse_sources_with_diagnostics`와 바이트 단위로 일치하며 전체 246개 project
test가 source span과 declaration ordering을 고정한다. P176e4b1은 deterministic project
name, source root와 ordered source path에서 source ID별 canonical package path/name을
계산하고 missing/mismatched `package` declaration을 Rust `build_package_graph`와 같은
stage, span, message로 거부한다. 세 package-layout fixture set, 163개 parser corpus source와
전체 248개 project test가 이 package identity boundary를 고정한다. P176e4b2a는 같은
root project의 source package를 묶고 file-local import path/qualifier 중복, unresolved target,
cycle을 Rust package graph와 같은 진단으로 거부하며 package/import를 lexical order로,
build order를 dependency-first로 정규화한다. valid 및 일곱 rejection layout set, 163개
parser corpus source와 전체 250개 project test가 이 경계를 고정한다. P176e4b2b는
struct/enum/function declaration을 package별로, method를 receiver별로 수집하고 visibility와
type parameter를 보존한다. 일반 declaration과 receiver method collision은 Rust와 같은
source span에서 거부한다. valid 및 아홉 rejection layout set, 163개 parser corpus source와
전체 251개 project test가 이를 고정한다. P176e4b2c는 source가 없는 compiler-owned
`std/errors`, `std/fs`, `std/io`, `std/os`, `std/strings`, `std/collections` package를 import
span에서 합성하고 30개 public declaration의 struct/enum/function/opaque kind와 generic
parameter를 Rust registry와 같은 lexical order로 보존한다. 존재하지 않는 `std/*` import는
Rust와 같은 span/message로 거부한다. 두 valid 및 열 rejection layout set, 163개 parser
corpus source와 전체 253개 project test가 이를 고정한다. P176e4b3a는 dependency project의
source root를 compiler input에 보존하고 project별 direct dependency edge만 import를
허용한다. `app -> text -> shared` transitive graph는 dependency-first로 정규화하며, `app`이
직접 선언하지 않은 `shared`를 import하면 Rust와 같은 source/span/message로 거부한다. 세
valid 및 열한 rejection layout set, 163개 parser corpus source와 전체 255개 project test가
이를 고정한다. P176e4b3b는 declaration/type/pattern/function value/call을 package-qualified
internal symbol로 rewrite하고 imported visibility/kind, public API와 receiver 경계를 Rust
linker와 같은 span/message로 검증한다. 다섯 focused linker test, 여섯 project differential
invocation과 전체 11-file compiler source link가 Stage0과 일치하며, 165개 parser corpus
source와 전체 260개 project test가 이 경계를 고정한다. P176e4c1은 linked program에
compiler-owned intrinsic type 3개와 function 27개를 주입하고 generic `Map` 및 collection
function specialization의 metadata와 use-site span을 보존한다. Semantic checker는 synthetic
intrinsic body를 건너뛰고 지원하지 않는 map key를 거부하며, typed IR은 intrinsic declaration을
제외한 채 direct call과 function value의 intrinsic identity를 유지한다. Valid project의
augment/prepare/check/IR 네 출력과 map-key rejection이 Stage0과 일치한다. 세 focused test,
다섯 project differential invocation, 167개 parser corpus source와 전체 263개 project test가
이를 고정한다. 후속 성능 개선에서 byte cursor의 반복 전체 UTF-8 검증을 제거하고
fixture/corpus를 최대 4개로 병렬화했다. `mlg test`는 선택된 test body를 하나의
deterministic C runner로 만들되 각 case를 별도 child process로 실행한다. 전체 263-test
compiler suite는 약 250초와 1.76 GB의 중복 C에서 3.2-3.4초와 9.9 MB artifact directory로
줄었다. 후속 IR closure에서 함수 단위 comparator와 약 20초 rebuild diagnostic을 추가하고,
assignment reactivation, borrow full-expression, computed place, partial field move, return
pre-evaluation과 Copy pattern shadow cleanup 차이를 6개 fixture로 고정했다. 12-file compiler
source의 link/prepare/check와 normalized typed IR 675개 함수가 모두 Stage0과 일치한다. C
backend는 pattern arm span 기반 binding identity를 사용해 inner Copy binding이 outer cleanup
drop을 가리키는 문제를 제거했다. Generated Stage1/accounting/sanitizer/oracle 빌드를 병렬화하고
fast mode의 24회 exact runner 재빌드를 전체 263-test 단일 실행으로 교체했다. 같은 local host에서
IR-focused gate는 26초, fast gate는 40초, complete 48-IR-fixture/167-source gate는 83초에
통과했다. Canonical repository gate와 public `main` publication, macOS arm64/Linux x86_64
CI acceptance도 통과해 B2를 닫았다. B3 P177a는 typed-IR read-only API,
Mallang scalar C emitter와 standalone `c` mode를 추가했다. Scalar fixture의
Stage0/Stage1 generated C는 byte-identical하고 strict native, allocation accounting,
ASan/UBSan을 통과한다. Compiler source 713개 normalized IR function도 Stage0과
일치한다. P177b1은 string runtime, UTF-8 literal/equality, owned string
return/local/drop, full-expression cleanup, statement `if`/`else`, condition
loop와 `break`/`continue`를 추가했다. Expanded compiler source 725개 normalized
IR function이 Stage0과 일치하고, scalar와 owned-control fixture는 모두
Stage0/Stage1 byte identity, strict native, accounting, ASan/UBSan을 통과한다.
P177b2는 semantic type shape와 specialized struct/enum declaration을 IR
metadata로 보존하고 fixed array, heap-backed slice, struct layout, composite
literal, field/index/len과 recursive drop helper를 추가했다. 세 positive
fixture와 bounds/unsupported-node rejection이 Stage0 byte identity, strict
native, accounting, ASan/UBSan을 통과하고 compiler source 806개 normalized IR
function이 일치한다. P177b3는 `Option`/`Result`, inline user enum과 recursive
owned enum의 definition/constructor/drop helper, expression `if`, 두 match form과
nested pattern binding environment를 Stage0와 동일하게 구현했다. `adt-match`는
multi-payload, nested inline/owned pattern, recursive node consumption과 string
cleanup을 실제 실행하고, 강제 recursive allocation failure와 `RangeFor` boundary를
포함한 4 positive/4 rejection 경로가 byte identity, strict native, accounting과
ASan/UBSan을 통과한다. 기존 recursive-enum, match-arm-prelude와 nested-pattern
예제도 Stage0 C와 byte-identical하며 compiler source 831개 normalized IR function이
일치한다. B3 전체는 아직 active이며 range/three-clause loop, overwrite cleanup,
slice append, dynamic owned string, callable/project surface와 complete compiler-source
C generation이 남아 있다.

B2 개발 루프는 generated Stage1과 strict accounting을 strict C11 `-O2`로,
ASan/UBSan 경로를 `-O1`로 실행한다. 수정 중에는
`scripts/diagnose-self-hosting-compiler-ir.sh --rebuild-bootstrap` 또는
`scripts/check-self-hosting-lexer.sh --focus <area>`, integration 시점에는 `--fast`,
publication 직전에는 인자 없는 full gate를 사용한다. `--jobs`는 기본 최대 4개의
bounded worker를 제어한다. `--fast`는
전체 repository source의 Stage0/Stage1 differential을 유지하면서 focused
accounting, 전체 263개 compiler project test의 단일 runner와
lexer/parser/semantic/typed-IR sanitizer smoke를 실행한다. 인자 없는 full gate는 complete
compiler-source link/prepare/check/IR와
corpus accounting/sanitizer를 실행하며 milestone, publication과 release evidence로
인정한다.

B3 backend 수정 중에는 fresh Stage1 artifact가 확인된 경우
`scripts/check-self-hosting-backend.sh --assume-bootstrap`을 사용한다. 관측된 edit
loop는 P177b3의 네 positive/네 rejection path 기준 약 10초다. Compiler
source 또는 Stage0가 바뀌면 인자 없는 backend gate로 Stage1을 다시 만들며
관측된 integration loop는 약 22-25초다. Ownership/typed-IR
변경은 compiler-source IR diagnostic을 추가하고, `scripts/check.sh`는 직전 full
self-hosting gate가 만든 Stage1을 재사용해 backend bootstrap 중복을 피한다. 시간은
host-local 관측값이며 gate threshold가 아니다.

Publish helper note: the real publish path fetches `origin` before verification
and again before bookmark movement, with Homebrew Git preferred when available,
and refuses to publish if `main@origin` no longer matches the local `main` base.
The `--no-push` dry run exercises the same freshness checks but stops before
bookmark movement and push. After real push, the helper fetches again and
verifies `main@origin` points at the published commit.
