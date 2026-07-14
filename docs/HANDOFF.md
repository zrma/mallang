# Mallang Handoff

## 현재 상태

- 언어 이름: Mallang
- 소스 확장자: `.mlg`
- CLI: `mlg`
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
- v0.3 구현 완료, release pending: 승인된 `func(T) U`, `func mut(T) U` function
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
- v0.4 구현 완료: generic struct/function declaration과 explicit type argument를
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
- v0.5 P145 완료, release pending: slice buffer, closure environment, recursive enum node와
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
- 아직 없음: first-class borrowed references, statement-spanning borrow lifetimes, general partial moves from fields beyond slice field take, full C backend, method values/interfaces/dynamic dispatch. `con expr` / `mut expr` remain call argument mode prefixes only; statement-spanning borrow syntax is explicitly deferred. Non-slice field partial moves remain explicitly deferred; owned slice field take is the only v0 field-take exception.

## 빠른 시작

```sh
scripts/check-agent-harness-interface.sh
scripts/check.sh
scripts/check-release-binary.sh
scripts/check-release-helpers.sh
scripts/check-generated-c-sanitizers.sh --assume-generated
scripts/verify-v0-rc.sh
scripts/finalize-and-push.sh --verify-only
scripts/finalize-and-push.sh --message "chore: publish mallang 0.1.0" --no-push
cargo run --bin mlg -- --version
cargo run --bin mlg -- --help
cargo run --bin mlg -- check examples/first.mlg
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
- `SPEC.md`: 언어 v0.1 스펙
- `docs/V1_ROADMAP.md`: `v0.2.0`부터 `v1.0.0`까지 아홉 개 장기 milestone과 완료 조건
- `docs/todo-v03-functions-closures/`: v0.3 function value와 owned closure decision gate
- `docs/todo-v04-generic-data-model/`: v0.4 generic enum과 static specialization decision gate
- `docs/todo-v05-ownership-runtime/`: v0.5 minimal ownership model과 transparent recursive ADT contract
- `docs/todo-v06-standard-library/`: approved v0.6 contract and P147-P152 implementation evidence
- `docs/releases/v0-rc.md`: v0.1.0 release notes와 verification record
- `ROADMAP.md`: compiler milestone
- `docs/ROADMAP.md`: agent가 다음 작업을 고르는 운영용 roadmap
- `docs/REPO_MANIFEST.yaml`: 검증 명령과 entrypoint 선언
- `docs/ESCALATION_POLICY.md`: 사용자 호출 조건

## 다음 구현 후보

1. P153에서 local supported-host와 Ubuntu CI native standard-library acceptance를 닫는다.
2. Standard API reference, `SPEC.md`, README, roadmap와 handoff를 implementation에 맞춘다.
3. v0.6 completion evidence와 v0.7 tooling/platform decision gate를 확정한다.

Publish helper note: the real publish path fetches `origin` before verification
and again before bookmark movement, with Homebrew Git preferred when available,
and refuses to publish if `main@origin` no longer matches the local `main` base.
The `--no-push` dry run exercises the same freshness checks but stops before
bookmark movement and push. After real push, the helper fetches again and
verifies `main@origin` points at the published commit.
