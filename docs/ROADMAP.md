# Agent Roadmap

## P0: Bootstrap

- [x] Mallang naming 정리
- [x] Rust crate 생성
- [x] lexer/token model 추가
- [x] repo 관리 문서와 검증 스크립트 추가
- [x] GitHub repo publish

## P1: Parser Frontend

- [x] AST module 추가
- [x] function declaration parser 추가
- [x] block/statement parser 추가
- [x] Pratt expression parser 추가
- [x] `else if` sugar parser 추가
- [x] `|>` pipeline call sugar parser/native smoke 추가
- [x] condition-only `for` statement parser/native smoke 추가
- [x] `break` / `continue` loop control parser/native smoke 추가
- [x] `for init; condition; post` clause loop parser/native smoke 추가
- [x] initless `for ; condition; post` clause loop parser/native smoke 추가
- [x] conditionless `for` / empty-condition clause loop parser/native smoke 추가
- [x] `examples/first.mlg`를 AST로 파싱하는 test 추가

## P2: Static Semantics

- [x] first native subset용 name resolver 추가
- [x] first native subset용 primitive type checker 추가
- [x] `string` equality semantic/backend/native smoke 추가
- [x] `bool` logical operator semantic/backend/native smoke 추가
- [x] first native subset용 function signature checker 추가
- [x] immutable binding reassignment reject
- [x] `if` expression type checking 추가
- [x] statement-form `if` type checking 추가
- [x] statement-form `if` return-completeness analysis 추가
- [x] condition-only `for` statement checking 추가
- [x] `for init; condition; post` header-local checking 추가
- [x] initless `for ; condition; post` checking 추가
- [x] conditionless `for` / empty-condition clause loop checking 추가
- [x] `break` / `continue` outside-loop reject 추가
- [x] built-in value name collision reject 추가
- [x] top-level type/function declaration name conflict reject 추가
- [x] statement-only `print` value-position reject 추가
- [x] `mlg check` subcommand 추가

## P3: Ownership Lite

- [x] Copy/move type classification 추가
- [x] use-after-move reject
- [x] `con` read borrow call rule 추가
- [x] `mut` exclusive borrow call rule 추가
- [x] same-call overlapping borrow tracking 추가
- [x] non-copy borrowed parameter return/storage/owned-arg escape reject 추가

## P4: Native Backend

- [x] typed IR 추가
- [x] `if` expression typed IR/codegen 추가
- [x] first native subset용 C codegen 추가
- [x] `mlg build` subcommand 추가
- [x] `clang` 기반 native binary smoke 추가
- [x] statement-form `if` C codegen/native smoke 추가
- [x] `con`/`mut` parameter hidden-reference C ABI 추가
- [x] prelude가 필요한 `if` expression branch용 C temp lowering 추가
- [x] prelude가 필요한 `match` expression arm용 C temp lowering 추가
- [x] `&&` / `||` short-circuit native smoke 추가
- [x] `|>` pipeline call sugar native smoke 추가
- [x] condition-only `for` statement C backend/native smoke 추가
- [x] `break` / `continue` C backend/native smoke 추가
- [x] `for init; condition; post` C backend/native smoke 추가
- [x] initless `for ; condition; post` C backend/native smoke 추가
- [x] conditionless `for` / empty-condition clause loop C backend/native smoke 추가

## P5: Built-in ADTs

- [x] `Option[T]` / `Result[T, E]` surface 설계
- [x] generic type reference parser 추가
- [x] `Some` / `None` / `Ok` / `Err` constructor type checking 추가
- [x] `Option` / `Result` exhaustive `match` 추가
- [x] tagged typed IR와 C backend layout 추가
- [x] printable payload를 가진 `Option` / `Result` native print 추가
- [x] non-local `match` scrutinee temp codegen 추가
- [x] statement-form `match` block arm 추가

## P6: Structs

- [x] `type Name struct { ... }` parser/semantic 추가
- [x] named struct literal과 field access 추가
- [x] struct typed IR와 C backend typedef/literal/access 추가
- [x] struct receiver methods 설계/구현
- [x] caller-visible `mut` receiver methods native smoke 추가
- [x] direct mutable field assignment 추가
- [x] field-level borrow arguments 추가
- [x] nested field assignment와 nested field borrow argument 추가
- [x] printable field를 가진 struct native print 추가

## P7: Arrays And Range

- [x] fixed-size array와 array-only `range`의 v0 surface 결정
- [x] `[N]T` type reference parser 추가
- [x] `[N]T{...}` fixed-size array literal parser 추가
- [x] fixed-size array semantic/type checking 추가
- [x] array-only `for i, value := range values { ... }` parser/semantic 추가
- [x] fixed-size array typed IR와 C struct-wrapper layout 추가
- [x] array-only `range` C backend/native smoke 추가
- [x] fixed array indexing/`len`을 slice 설계와 분리하고 slice `[]T`,
  append/growth, mutable range는 후속 slice로 보류

## P8: Fixed Array Indexing And Len

- [x] `values[i]` indexing expression parser 추가
- [x] fixed-size array indexing semantic/type checking 추가
- [x] `len(values)` fixed-size array built-in semantic 추가
- [x] fixed-size array indexing typed IR와 C backend 추가
- [x] fixed-size array `len` typed IR와 C backend 추가
- [x] native smoke에서 range 이후 source 재사용, indexing, `len` 검증

## P9: Fixed Array Bounds Safety

- [x] literal out-of-bounds fixed array indexes를 `mlg check`에서 reject
- [x] non-literal fixed array indexes에 native runtime bounds guard 추가
- [x] runtime guard codegen에서 base/index expression 중복 평가 방지
- [x] native smoke에서 dynamic in-bounds index 경로 검증

## P10: Fixed Array Element Assignment

- [x] `values[i] = expr` statement parser 추가
- [x] mutable fixed-size array binding/parameter에만 element assignment 허용
- [x] Copy element assignment semantic 경로 추가
- [x] assignment index compile-time/runtime bounds check 적용
- [x] typed IR와 C backend에서 checked element assignment 추가
- [x] native smoke에서 assignment 이후 range/index/len 결과 검증

## P11: Fixed Array For-Post Assignment

- [x] `for ...; ...; values[i] = expr` post parser 추가
- [x] 기존 fixed array element assignment semantic 규칙을 for post에 재사용
- [x] typed IR에서 index assignment target을 for post로 lowering
- [x] C `for` header에서 사용할 수 있는 runtime bounds helper 추가
- [x] native smoke에서 for post array assignment 실행 결과 검증

## P12: Prefix Parameter Modes

- [x] read borrow keyword를 `in`에서 `con`으로 교체
- [x] parameter/receiver mode를 `con name T` / `mut name T` prefix로 고정
- [x] call argument mode를 `con expr` / `mut expr`로 고정
- [x] suffix mode 없이 prefix grammar만 지원
- [x] examples/docs/tests를 canonical borrow syntax로 갱신

## P13: For-Clause Prelude Lowering

- [x] `for` clause condition에서 prelude가 필요한 expression lowering 지원
- [x] `for` clause post assignment target/RHS에서 prelude가 필요한 lowering 지원
- [x] post가 있는 `for` clause body의 `continue`를 post label로 lowering
- [x] native smoke에서 condition/post prelude와 `continue` post 실행 검증

## P14: Array Element Borrow Arguments

- [x] borrow argument place에 direct fixed array element path 추가
- [x] non-copy array element를 `con`/`mut` function argument로 lowering
- [x] backend에서 array element borrow를 checked lvalue address로 lowering
- [x] native smoke에서 `con users[i].field`와 `mut users[i].field` 검증

## P15: Non-Copy Array Element Assignment

- [x] Copy-only assignment guard 제거
- [x] owned RHS move semantics 유지
- [x] for-post non-copy index target lowering 지원
- [x] native smoke에서 struct element replacement 검증

## P16: Array Element Method Receivers

- [x] receiver method lookup을 direct local/field/index borrow place type으로 확장
- [x] `con`/`mut` receiver borrow와 explicit argument overlap check 공유
- [x] IR에서 array element receiver를 borrow argument lowering으로 처리
- [x] native smoke에서 `counters[i].inc()` caller-visible mutation 검증

## P17: Slice Type Surface

- [x] `[]T` type reference parser 추가
- [x] AST에서 fixed-size array `[N]T`와 slice `[]T` 구분
- [x] semantic checker에서 `[]T` reserved diagnostic 고정
- [x] slice values/native ABI는 후속 ownership decision으로 보류

## P18: Array Range Blank Identifier

- [x] `for _, value := range values` parser/semantic 지원
- [x] `for i, _ := range values`에서 value binding 생략
- [x] value blank range에서 non-copy element copy requirement 제거
- [x] native smoke에서 blank index/value codegen 검증

## P19: Array Range One Variable

- [x] `for i := range values` parser/semantic 지원
- [x] one-variable range를 value blank range로 lowering
- [x] non-copy element array를 index-only range로 순회
- [x] native smoke에서 value copy 없는 codegen 검증

## P20: mlg Run Command

- [x] `mlg run <source-file>` CLI 추가
- [x] `build`와 `run`의 native compile path 공유
- [x] `target/mallang/run/<source-stem>` binary 실행
- [x] native smoke에서 program stdout 검증

## P21: Bool Unary Not

- [x] `!expr` parser precedence 고정
- [x] semantic checker에서 `!` operand/result type 검증
- [x] typed IR와 C backend에서 `UnaryOp::Not` 검증
- [x] native smoke에서 `!`와 short-circuit operator 조합 검증

## P22: Integer Division Zero Safety

- [x] literal `/ 0` and `% 0` semantic reject 추가
- [x] dynamic divisor를 C temp로 한 번만 평가
- [x] native C에서 zero divisor runtime guard 추가
- [x] native smoke에서 정상 `/`/`%`와 zero divisor 실패 검증

## P23: Checked Integer Arithmetic

- [x] literal `+`, `-`, `*`, unary `-`, `/`, `%` overflow semantic reject 추가
- [x] dynamic `+`, `-`, `*`, unary `-`를 checked C builtin으로 lowering
- [x] dynamic `INT64_MIN / -1`와 `INT64_MIN % -1` runtime guard 추가
- [x] native smoke에서 정상 산술과 overflow 실패 검증

## P24: Recursive Struct Type Check

- [x] direct recursive struct value type semantic reject 추가
- [x] indirect recursive struct value type semantic reject 추가
- [x] `Option`/`Result`/fixed array wrapper 안의 recursive struct reference reject 추가
- [x] `mlg check` failure smoke 추가

## P25: Printability Semantic Check

- [x] `print` 가능 타입 집합을 semantic checker에 고정
- [x] fixed-size array `print` semantic reject 추가
- [x] non-printable payload/field를 가진 `Option`/`Result`/`struct` print reject 추가
- [x] `mlg check` failure smoke 추가

## P26: Built-in Value Name Reservation

- [x] global function 이름이 built-in value name과 충돌하면 reject
- [x] parameter/local/range binding 이름이 built-in value name과 충돌하면 reject
- [x] match payload binding 이름이 built-in value name과 충돌하면 reject
- [x] `mlg check` failure smoke 추가

## P27: Top-Level Declaration Namespace

- [x] top-level struct와 non-method function 이름 충돌 reject
- [x] top-level declaration에서 built-in type/value 이름 충돌 reject
- [x] concrete method 이름은 receiver-qualified namespace로 유지
- [x] `mlg check` failure smoke 추가

## P28: Shadowing Scope Semantics

- [x] same-block binding redeclaration reject 유지
- [x] nested block shadowing 허용
- [x] shadowed inner binding move가 outer binding move로 merge되지 않게 고정
- [x] `for`/`range` body shadowing을 위한 native C block lowering 추가
- [x] native smoke에서 nested shadowing 출력 검증

## P29: Control-Flow Scope Regression

- [x] `match` expression payload shadowing native smoke 추가
- [x] statement `match` payload move가 outer binding move로 merge되지 않는 semantic test 추가
- [x] condition-only `for` body shadowing semantic/native smoke 추가
- [x] `examples/shadowing.mlg`로 control-flow scope isolation 회귀 고정

## P30: Append Built-in Reservation

- [x] `append`를 future slice growth built-in value name으로 예약
- [x] top-level function/local binding 충돌 semantic regression 추가
- [x] `mlg check` failure smoke 갱신

## P31: Entrypoint Signature Semantics

- [x] `func main()`을 v0 entrypoint signature로 고정
- [x] `main` method receiver, parameter, return type reject 회귀 테스트 추가
- [x] `mlg check` failure smoke로 invalid entrypoint signature 고정

## P32: Slice Reserved Boundary Regression

- [x] `[]T` direct parameter reserved diagnostic 유지
- [x] return type, struct field, generic payload, fixed-array element 안의 `[]T`
      reserved diagnostic 회귀 테스트 추가
- [x] `mlg check` failure smoke로 nested slice reserved boundary 고정

## P33: Slice Ownership And Append Decision

- [x] `[]T`를 Go-style aliasing header가 아닌 owned move-only growable buffer로 결정
- [x] slice native ABI를 `{ data, len, cap }` 형태의 compiler-owned heap resource로 결정
- [x] `append(values, item)`을 첫 slice 인자를 소비하고 새 owned slice를 반환하는 built-in으로 결정
- [x] slice implementation 선행 조건을 cleanup/drop lowering으로 고정

## P34: Internal Slice Type Shell

- [x] `Type::Slice(T)` internal type shell 추가
- [x] slice type을 non-copy cleanup resource로 분류
- [x] C backend에서 internal `{ data, len, cap }` slice typedef emission 추가
- [x] user-facing `[]T` semantic reserved diagnostic 유지

## P35: Cleanup Drop Helper Shell

- [x] cleanup type별 C backend `mlg_drop_*` helper emission 추가
- [x] internal slice drop helper에서 owned buffer free와 header reset 수행
- [x] `Option`/`Result`/array wrapper cleanup helper가 active payload/element drop helper를 호출
- [x] actual scope exit, early return, reassignment drop insertion은 다음 단계로 유지

## P36: Explicit Drop IR Backend

- [x] `IrStmtKind::Drop` explicit cleanup statement 추가
- [x] C backend에서 cleanup lvalue를 `mlg_drop_*(&place)`로 lowering
- [x] non-cleanup type drop은 IR invariant error로 reject
- [x] automatic scope exit/early return drop insertion은 다음 단계로 유지

## P37: Straight-Line Cleanup Drop Insertion

- [x] owned cleanup parameters를 active cleanup roots로 추적
- [x] straight-line cleanup locals를 active cleanup roots로 추적
- [x] function tail과 top-level `return` 전에 `IrStmtKind::Drop` 삽입
- [x] returned cleanup root는 drop 대상에서 제외
- [x] branch/loop/reassignment cleanup insertion은 다음 단계로 유지

## P38: Straight-Line Cleanup Reassignment Drop

- [x] active cleanup root reassignment 전에 old value `IrStmtKind::Drop` 삽입
- [x] reassignment RHS로 move된 cleanup root는 active roots에서 제거
- [x] reassigned cleanup root는 새 value cleanup 대상으로 유지
- [x] branch/loop control-flow cleanup insertion은 다음 단계로 유지

## P39: Branch-Local Cleanup Drop Insertion

- [x] `if` statement then/else body에 branch-local cleanup drop insertion 적용
- [x] statement-form `match` arm body에 arm-local cleanup drop insertion 적용
- [x] branch-local cleanup roots는 arm tail 또는 arm-local return 전에 drop
- [x] outer cleanup root branch moves와 loop cleanup insertion은 다음 단계로 유지

## P40: Branch Outer Cleanup Move Normalization

- [x] `if` condition에서 move된 cleanup root를 parent active roots에서 제거
- [x] `if` branch 중 하나에서 move된 outer cleanup root를 다른 continuing branch tail에서 drop
- [x] statement-form `match` scrutinee/arm move도 같은 merge-drop 규칙 적용
- [x] branch-local `return` 전에 outer cleanup root drop 삽입
- [x] loop cleanup insertion은 다음 단계로 유지

## P41: Loop Body-Local Cleanup Drop Insertion

- [x] `for` body-local cleanup roots를 loop body tail에서 drop
- [x] `range` body-local cleanup roots를 loop body tail에서 drop
- [x] `break`/`continue` 전에 loop body-local cleanup roots drop
- [x] loop body 안의 `return` 전에 outer cleanup roots와 body-local roots drop
- [x] outer cleanup root loop moves와 for-init cleanup은 다음 단계로 유지

## P42: For-Init Cleanup Trailer

- [x] `IrStmtKind::For`에 loop-exit cleanup trailer 추가
- [x] cleanup type `for` init binding을 loop cleanup root로 추적
- [x] normal loop exit와 `break` 이후 for-init cleanup root drop
- [x] loop body `return` 전 for-init cleanup root drop
- [x] loop body에서 for-init root가 move되는 runtime state tracking은 다음 단계로 유지

## P43: Loop Persistent Move Safety

- [x] `for` condition/body/post에서 loop-persistent move-only binding move reject
- [x] three-clause `for` init binding move reject
- [x] `range` body에서 outer move-only binding move reject
- [x] loop body-local move-only binding move는 허용 유지
- [x] runtime moved-state tracking 대신 v0 정적 제한으로 cleanup safety 유지

## P44: Field/Index Cleanup Overwrite Drop

- [x] cleanup type field assignment 앞에 old field drop 삽입
- [x] cleanup type fixed-array element assignment 앞에 old element drop 삽입
- [x] backend explicit field/index drop lvalue lowering regression 추가
- [x] source-level slice surface는 reserved 상태 유지

## P45: Cleanup Assignment RHS Before Drop

- [x] cleanup type local reassignment에서 RHS temp 평가를 old root drop보다 먼저 삽입
- [x] cleanup type field assignment에서 RHS temp 평가를 old field drop보다 먼저 삽입
- [x] cleanup type fixed-array element assignment에서 RHS temp 평가를 old element drop보다 먼저 삽입
- [x] source-level slice surface는 reserved 상태 유지

## P46: Expression Branch Cleanup Normalization

- [x] expression-form `if` branch cleanup trailer 추가
- [x] expression-form `match` arm cleanup trailer 추가
- [x] expression branch별 cleanup root move merge/drop normalization 추가
- [x] C backend에서 expression cleanup trailer를 temp block으로 lowering
- [x] source-level slice surface는 reserved 상태 유지

## P47: Owned Slice Literal/Len/Index Surface

- [x] source-level `[]T`를 owned move-only slice type으로 허용
- [x] `[]T{...}` slice literal semantic/IR/backend lowering 추가
- [x] `len(slice)` read-only builtin surface 추가
- [x] Copy-only `slice[i]` value access와 native bounds check 추가
- [x] slice range, slice element borrow는 후속 work로 유지

## P48: Slice Append Built-in

- [x] `append(slice, item)` semantic을 consuming owned slice builtin으로 추가
- [x] `values = append(values, item)` 재할당 후 cleanup root 재활성화
- [x] typed IR `SliceAppend`와 native C realloc growth lowering 추가
- [x] `examples/slice-append.mlg` native smoke 추가
- [x] slice element borrow는 후속 work로 유지

## P49: Slice Range

- [x] owned slice를 `range` source로 허용
- [x] Copy value binding과 index-only non-Copy iteration 지원
- [x] inline slice range source는 temporary cleanup 전까지 reject
- [x] range body에서 active range source reassignment reject
- [x] native C backend에서 slice header `mlg_len` 기반 loop lowering 추가
- [x] `examples/slice-range.mlg` native smoke 추가
- [x] slice element borrow는 후속 work로 유지

## P50: Slice Element Borrow

- [x] direct local slice source의 `con values[i]` / `mut values[i]` 허용, P55에서 local-rooted source로 확장
- [x] slice element field path borrow, 예: `con users[i].name`, 지원
- [x] same-root indexed borrow overlap을 array와 같은 conservative rule로 검증
- [x] native C backend에서 `mlg_len` bounds guard 뒤 hidden-reference argument lowering
- [x] `examples/slice-element-borrow.mlg` native smoke 추가
- [x] borrowed indexing expression, slice element assignment, mutable range values는 후속 work로 유지

## P51: Slice Element Assignment

- [x] direct mutable slice source의 `values[i] = expr` 허용
- [x] Copy/non-copy element RHS를 owned value로 slice slot에 move
- [x] native C backend에서 `mlg_len` bounds guard 뒤 element assignment lowering
- [x] cleanup element overwrite 시 RHS temp, old element drop, slot assignment 순서 보존
- [x] `examples/slice-element-assignment.mlg` native smoke 추가
- [x] indexed field assignment, borrowed indexing expression, mutable range values는 후속 work로 유지

## P52: Indexed Field Assignment

- [x] local-rooted array/slice element field path assignment 허용
- [x] nested indexed field path, 예: `users[i].profile.name = expr`, 지원
- [x] non-Copy indexed element를 value extraction 없이 assignment target IR로 lowering
- [x] native C backend에서 array/slice indexed lvalue field assignment lowering
- [x] `examples/indexed-field-assignment.mlg` native smoke 추가
- [x] borrowed indexing expression은 P53에서 완료, mutable range values는 후속 work로 유지

## P53: Borrowed Indexing Expressions

- [x] `ValueUse::Borrow` index expression에서 non-Copy array/slice element inspection 허용
- [x] `ValueUse::Owned` index extraction은 기존 Copy requirement 유지
- [x] indexed element의 non-Copy field move는 계속 reject
- [x] native C backend에서 read-only indexed field access smoke 추가
- [x] `examples/indexed-field-read.mlg` native smoke 추가
- [x] first-class references, statement-spanning borrow lifetimes, mutable range values는 후속 work로 유지

## P54: Struct Cleanup for Slice Fields

- [x] struct field의 `[]T` reject 제거
- [x] `Type::Struct`를 cleanup-capable root로 분류
- [x] C backend에서 struct drop helper가 cleanup field helper를 호출
- [x] struct local/reassignment/owned parameter cleanup insertion에 기존 cleanup pipeline 재사용
- [x] `examples/struct-slice-field.mlg` native smoke 추가
- [x] first-class references, statement-spanning borrow lifetimes, mutable range values는 후속 work로 유지

## P55: Local-Rooted Slice Field Reads

- [x] slice source 제약을 direct local에서 local-rooted place로 완화
- [x] `len(bag.values)`, Copy `bag.values[i]`, `range bag.values` 허용
- [x] `con bag.values[i]` / `mut bag.values[i]` borrow argument 허용
- [x] inline slice temporary reject는 유지
- [x] `examples/slice-field-read.mlg` native smoke 추가
- [x] consuming `append(bag.values, item)`은 P57에서 direct field path same-field
      reassignment로 제한해 완료

## P56: Local-Rooted Slice Field Element Assignment

- [x] indexed assignment source 제약을 direct local에서 local-rooted place로 완화
- [x] `bag.values[i] = expr` 허용
- [x] cleanup element overwrite에서 RHS temp, old element drop, slot assignment 순서 유지
- [x] native C backend에서 local-rooted indexed lvalue assignment lowering
- [x] `examples/slice-field-assignment.mlg` native smoke 추가
- [x] consuming `append(bag.values, item)`은 P57에서 direct field path same-field
      reassignment로 제한해 완료

## P57: Direct Slice Field Append Reassignment

- [x] `bag.values = append(bag.values, item)` 허용
- [x] `shelf.bag.values = append(shelf.bag.values, item)`처럼 indexed segment가
      없는 direct field path 허용
- [x] 같은 field append에서는 cleanup overwrite drop을 생략해 realloc된 source를
      double-drop하지 않도록 함
- [x] `grown := append(bag.values, item)` 같은 field source append는 P59에서
      source field take로 제한해 완료
- [x] `examples/slice-field-append.mlg` native smoke 추가

## P58: Indexed Slice Field Append Reassignment

- [x] `store.bags[i].values = append(store.bags[i].values, item)` 허용
- [x] matched path의 index expression은 stable expression으로 제한
- [x] local-rooted slice indexed field assignment source 제약을 direct local slice에서
      local-rooted slice place로 완화
- [x] 같은 indexed field append에서는 cleanup overwrite drop을 생략
- [x] mismatched source index와 call index는 P59에서 field take source로 허용
- [x] `examples/indexed-slice-field-append.mlg` native smoke 추가

## P59: Slice Field Take Append Source

- [x] `grown := append(bag.values, item)` 허용
- [x] `append(store.bags[i].values, item)`처럼 local-rooted indexed field source 허용
- [x] append result가 consumed buffer를 소유하고 source field는 empty slice로 reset
- [x] direct local `values` append move semantics는 유지
- [x] native backend에서 field source lvalue를 copy한 뒤 empty slice header를 write
- [x] `examples/slice-field-take-append.mlg` native smoke 추가

## P60: Owned Slice Field Take Expressions

- [x] `taken := bag.values`처럼 owned value position에서 slice field take 허용
- [x] `consume(bag.values)`처럼 owned parameter argument에서 slice field take 허용
- [x] `len(bag.values)`, `bag.values[i]`, `range bag.values` read source는 take 없이 유지
- [x] typed IR에 explicit slice field take node를 추가해 read와 move를 분리
- [x] native backend에서 take source lvalue를 temp slice header로 copy한 뒤 empty slice header를 write
- [x] `examples/slice-field-take.mlg` native smoke 추가

## P61: Backend C Module Split

- [x] backend public API를 `src/backend/mod.rs`로 고정
- [x] C backend implementation을 `src/backend/c.rs`로 분리
- [x] existing `generate_c` / `generate_c_from_ir` public re-export 유지
- [x] CLI와 tests가 backend module split 뒤에도 같은 API를 사용하게 유지

## P62: C Backend Name Helpers Split

- [x] C identifier/type-name/operator helper utilities를 `src/backend/c/names.rs`로 분리
- [x] `Type` C name, parameter ABI type, ADT constructor name, operator spelling helper를 names module로 이동
- [x] 기존 `generate_c` / `generate_c_from_ir` API와 C output behavior 유지

## P63: C Backend Type Emitter Split

- [x] type collection과 `typedef` emission을 `src/backend/c/types.rs`로 분리
- [x] cleanup-capable type의 `mlg_drop_*` helper emission을 type emitter module로 이동
- [x] `CGenerator`는 C output orchestration, function/statement/expression emission 책임 유지
- [x] 기존 `generate_c` / `generate_c_from_ir` API와 C output behavior 유지

## P64: C Backend Statement Emitter Split

- [x] statement/loop/match statement/print emission을 `src/backend/c/statements.rs`로 분리
- [x] `emit_stmt_with_env`와 cleanup statement emission만 parent expression/orchestration에서 호출 가능한 module-visible boundary로 유지
- [x] `CGenerator`의 expression emission은 기존 `src/backend/c.rs`에 남겨 후속 expression emitter split 후보로 유지
- [x] 기존 `generate_c` / `generate_c_from_ir` API와 C output behavior 유지

## P65: C Backend Expression Emitter Split

- [x] expression/literal/call/borrow-lvalue/match-expression emission을 `src/backend/c/expressions.rs`로 분리
- [x] statement emitter가 쓰는 `emit_stmt_expr_with_env` / `emit_borrow_lvalue_expr`만 module-visible boundary로 유지
- [x] `CGenerator`의 public `generate_c` / `generate_c_from_ir` API와 C output behavior 유지

## P66: C Backend Utility Helper Split

- [x] shared formatting, temp-name, checked-int helper, and parameter-env utilities를 `src/backend/c/utils.rs`로 분리
- [x] `c.rs`는 C output orchestration과 `CGenerator` boundary 중심으로 축소
- [x] statement/type/expression emitters가 utility helpers를 `utils` module 경유로 사용하게 정리
- [x] 기존 `generate_c` / `generate_c_from_ir` API와 C output behavior 유지

## P67: C Backend Unit Test Module Split

- [x] `src/backend/c.rs`의 C backend unit tests를 `src/backend/c/tests.rs`로 분리
- [x] `c.rs` production module을 C output orchestration과 module boundary 중심으로 유지
- [x] C backend test names와 `backend::c::tests::*` path 유지
- [x] 기존 `generate_c` / `generate_c_from_ir` API와 C output behavior 유지

## P68: Mutable Range Values Deferred

- [x] `for i, mut value := range values` syntax를 v0에서 계속 reject하도록 parser regression 추가
- [x] 기존 range value binding이 immutable local임을 semantic regression으로 고정
- [x] SPEC에 mutable range design이 copied local mutation, element borrow, indexed assignment 중 하나를 나중에 결정해야 한다고 명시
- [x] by-reference range iteration은 별도 future design 후보로 유지

## P69: By-Reference Range Iteration Deferred

- [x] `for i, con value := range values` syntax를 v0에서 계속 reject하도록 parser regression 추가
- [x] range binding syntax가 `con`/`mut` marker를 모두 받지 않는다고 SPEC에 명시
- [x] future borrowed range iteration은 statement-spanning borrow lifetime 설계 뒤에 열도록 고정

## P70: General Field Partial Moves Deferred

- [x] owned slice field take만 v0 field-take 예외로 유지
- [x] non-slice cleanup field move, 예: `profile := user.profile`, reject semantic regression 추가
- [x] partial-move/destructuring 설계 전까지 parent struct를 부분 초기화 상태로 만들지 않는다고 SPEC에 명시

## P71: Statement-Spanning Borrows Deferred

- [x] `borrowed := con user.name` 같은 borrow marker value-position syntax reject parser regression 추가
- [x] `return mut name` 같은 first-class mutable borrow return 후보 syntax reject parser regression 추가
- [x] `con expr` / `mut expr`가 call argument mode prefix일 뿐 general expression이 아니라고 SPEC에 명시

## P72: C Backend Runtime Error Helper

- [x] generated C에 `mallang_runtime_error(const char *message)` helper 추가
- [x] integer/index/slice allocation runtime guard emission을 helper 호출로 통합
- [x] generated C에 direct `fprintf(stderr, ...)` runtime failure emission이 helper 하나로 모였는지 regression 추가

## P73: Native Runtime Failure Stderr Smoke

- [x] `scripts/check.sh` runtime failure smoke가 non-zero exit뿐 아니라 stderr message도 검증
- [x] division/remainder, checked integer overflow, array bounds failure stderr를 `mallang runtime error: ...`로 고정
- [x] compile-time negative smoke와 runtime negative smoke 검증 경계를 분리

## P74: C Backend IR Invariant Regression

- [x] malformed `print` call arity IR가 C emission 전에 invariant error로 실패하는지 고정
- [x] non-array/slice `range` source IR가 invariant error로 실패하는지 고정
- [x] Option match에 Result arm이 섞인 malformed IR를 invariant error로 고정
- [x] `con`/`mut` borrow argument가 lvalue가 아닌 malformed IR를 invariant error로 고정

## P75: Slice Literal Allocation-Size Guard

- [x] non-empty native slice literal lowering에 `UINT64_MAX / sizeof(T)` allocation-size guard 추가
- [x] slice literal allocation-size overflow와 allocation failure가 모두 `mallang_runtime_error(...)` helper로 실패하는지 backend regression 추가
- [x] SPEC에서 native slice literal과 `append` allocation failure/overflow policy를 같은 runtime-error policy로 고정

## P76: Indexed Slice Field Append-Take Regression

- [x] `append(store.bags[i].values, item)` C lowering이 indexed field source를 temp slice header로 copy하는지 고정
- [x] consumed indexed source field를 empty slice header로 reset하는 codegen regression 추가
- [x] append result와 owning store cleanup ownership이 유지되는지 backend assertion 추가

## P77: Borrow Mode Alias Rejection Regression

- [x] `name in T` suffix read-borrow parameter form을 계속 reject하도록 parser regression 추가
- [x] `name mut T` suffix mutable-borrow parameter form을 계속 reject하도록 parser regression 추가
- [x] `in expr` call-site borrow alias를 열지 않도록 parser regression 추가

## P78: Slice Cleanup Spec Refresh

- [x] `SPEC.md`의 slice cleanup 설명을 future staging 문구에서 current implemented model로 갱신
- [x] deferred slice/borrow rules와 implemented cleanup model을 분리
- [x] ROADMAP에 v0 freeze 문서 정리 항목 기록

## P79: CLI Version Smoke

- [x] `mlg --version`이 Cargo package version을 출력하도록 추가
- [x] usage output에 `--version` form 추가
- [x] `scripts/check.sh`에서 `Cargo.toml` version과 CLI 출력 일치 smoke 추가

## P80: CLI Help and Error Stream Smoke

- [x] `mlg --help`가 성공 help를 stdout으로 출력하도록 정리
- [x] no-args usage는 stderr와 non-zero exit로 유지
- [x] unknown subcommand diagnostic을 stderr와 non-zero exit로 smoke 추가

## P81: V0 Release Candidate Audit

- [x] `examples/*.mlg` 전체가 `scripts/check.sh` smoke에 연결되어 있는지 확인
- [x] 새 예제가 smoke 밖으로 빠지면 `scripts/check.sh`가 실패하도록 guard 추가
- [x] v0 완료 범위와 post-v0 deferred boundary를 audit spec으로 고정

## P82: Generated C Sanitizer Smoke

- [x] cleanup-heavy generated C 예제를 ASan/UBSan으로 재컴파일
- [x] sanitizer 실행 stdout을 기존 native smoke 기대값과 맞춰 검증
- [x] sanitizer stderr가 비어 있어야 통과하도록 고정

## P83: Generated C Warning Clean Smoke

- [x] generated C runtime helper를 maybe-unused로 표시해 unused-function warning 제거
- [x] source-level unused parameter가 C warning이 되지 않도록 `(void)param;` emit
- [x] `continue`가 없는 `for` post loop에서는 unused label을 emit하지 않음
- [x] 대표 generated C 파일을 `clang -std=c11 -Wall -Wextra -Werror`로 검증

## P84: Deep Generated C Sanitizer Sweep

- [x] `scripts/check.sh`의 정상 generated C 예제 label을 source of truth로 재사용
- [x] 전체 정상 generated C 예제를 ASan/UBSan으로 재컴파일하고 실행
- [x] sanitizer 실행 stdout을 normal native binary stdout과 비교
- [x] default smoke와 분리된 explicit pre-publication gate로 문서화

## P85: Full Generated C Warning Clean Gate

- [x] `scripts/check.sh`의 정상 generated C 예제 label 전체를 warning-clean source of truth로 사용
- [x] conservative drop helper emission이 unused-function warning을 만들지 않도록 `MLG_UNUSED` 적용
- [x] range source/value temp가 source-level 미사용 binding 때문에 warning을 만들지 않도록 처리
- [x] `scripts/check.sh` default gate에서 전체 generated C를 `clang -std=c11 -Wall -Wextra -Werror`로 검증

## P86: V0 RC Pre-Publish Verification

- [x] remote publish 전 로컬 v0 RC 검증 명령 `scripts/verify-v0-rc.sh` 추가
- [x] normal smoke, deep generated C sanitizer, roadmap completion, local stack, attribution을 한 번에 확인
- [x] `--skip-deep-sanitizers` fast rerun 경로 추가
- [x] remote publish는 사용자 승인 필요 gate로 유지

## P87: Publish Finalizer RC Gate

- [x] `scripts/finalize-and-push.sh`가 bookmark 이동 전 `scripts/verify-v0-rc.sh`를 실행하도록 연결
- [x] remote publish 없이 finalization flow를 검증하는 `--no-push` 추가
- [x] 실제 bookmark 이동과 remote push는 명시적 finalizer invocation에만 남김
- [x] README/manifest/handoff에 approval-gated publish 경로 기록

## P88: V0 RC Release Notes

- [x] `docs/releases/v0-rc.md`에 v0 local release-candidate 범위 기록
- [x] language surface, safety model, native backend gate, CLI, verification command 정리
- [x] post-v0 deferred boundary와 approval-gated publish command 기록
- [x] README/HANDOFF에서 release note를 찾을 수 있게 연결

## P89: Publish Verify-Only Finalizer

- [x] `scripts/finalize-and-push.sh --verify-only` 추가
- [x] verify-only는 jj description, bookmark, remote를 변경하지 않고 v0 RC gate만 실행
- [x] README/HANDOFF/release note/manifest에 side-effect-free publish readiness command 기록
- [x] 기존 `--no-push`는 final jj description까지 쓰는 dry-run 경로로 명확히 문서화

## P90: Release Helper Contract Checks

- [x] `scripts/check-release-helpers.sh` 추가
- [x] release helper shell syntax/help/invalid option contract를 경량 검증
- [x] `--verify-only`가 `--message`/`--bookmark`와 결합될 수 없음을 자동 검증
- [x] `scripts/verify-v0-rc.sh`가 release helper contract check를 먼저 실행하도록 연결

## P91: Publish Remote Freshness Guard

- [x] real publish 경로에서 bookmark 이동 전 `jj git fetch --remote origin` 실행
- [x] `jj git fetch/push` 경로에서 가능하면 Homebrew Git을 우선 사용
- [x] fetch 후 `main@origin`이 local `main` base와 다르면 publish 중단
- [x] `--verify-only`와 `--no-push`는 description/bookmark/remote side effect 없는 기존 경계 유지
- [x] README/HANDOFF/release note에 stale remote guard 기록

## P92: Publish Freshness Preflight

- [x] real publish 경로에서 final description 작성 전 remote freshness preflight 실행
- [x] full v0 RC verification 후 bookmark 이동 직전 remote freshness final check 유지
- [x] stale remote이면 expensive verification이나 local description mutation 전에 먼저 중단
- [x] release helper contract check가 preflight/final freshness wiring을 검증

## P93: No-Push Freshness Dry Run

- [x] `--no-push` finalization dry run에서 remote freshness preflight/final check 실행
- [x] `--no-push`는 final jj description과 v0 RC gate를 검증하되 bookmark 이동/push는 계속 생략
- [x] `--verify-only`는 description/bookmark/remote freshness side effect 없는 readiness gate로 유지
- [x] release helper contract check가 freshness flag wiring을 검증

## P94: Finalizer Option Value Diagnostics

- [x] `--message` 값 누락/빈 값/다음 옵션 토큰을 exit 2와 명확한 usage로 거부
- [x] `--bookmark` 값 누락/빈 값을 exit 2와 명확한 usage로 거부
- [x] release helper contract check가 missing/empty option value failure를 검증
- [x] 기존 invalid message format failure와 publish approval gate 유지

## P95: Publish Post-Push Verification

- [x] real publish 경로에서 bookmark 이동 전 `@` commit을 publish target으로 저장
- [x] push 후 `origin`을 다시 fetch하고 remote bookmark가 publish target을 가리키는지 비교
- [x] remote bookmark mismatch는 명확한 diagnostic과 non-zero exit로 처리
- [x] release helper contract check가 post-push verification wiring을 검증

## P96: Release Binary Smoke

- [x] `scripts/check-release-binary.sh` 추가
- [x] `cargo build --release --bin mlg` 후 `target/release/mlg --version`/`--help` 검증
- [x] release binary로 `check examples/first.mlg`와 native `build`/run smoke 검증
- [x] `scripts/verify-v0-rc.sh`에 release binary smoke 연결

## P97: Release Binary Run Smoke

- [x] `scripts/check-release-binary.sh`가 `target/release/mlg run examples/first.mlg`를 직접 검증
- [x] release `mlg run` stdout이 `30`인지 확인
- [x] 기존 `scripts/verify-v0-rc.sh` release binary smoke 경로로 P97 검증 포함

## P98: Release Binary Frontend Smoke

- [x] `target/release/mlg lex examples/first.mlg` smoke 추가
- [x] `target/release/mlg parse examples/first.mlg` smoke 추가
- [x] `target/release/mlg ir examples/first.mlg` smoke 추가
- [x] 기존 `scripts/verify-v0-rc.sh` release binary smoke 경로로 P98 검증 포함

## P99: Release Binary Safety Rejection Smoke

- [x] release binary `check`가 use-after-move source를 reject하는지 검증
- [x] release binary `check`가 borrowed non-copy escape source를 reject하는지 검증
- [x] release binary `check`가 same-call overlapping borrow source를 reject하는지 검증
- [x] failure stdout은 비어 있고 stderr에 안정적인 safety diagnostic이 있는지 검증

## P100: Release Binary CLI Error Smoke

- [x] `target/release/mlg` no-args invocation이 stderr usage와 non-zero exit를 내는지 검증
- [x] `target/release/mlg nope` unknown subcommand diagnostic과 non-zero exit를 검증
- [x] release CLI failure stdout이 비어 있는지 검증
- [x] 기존 safety rejection smoke도 공통 failure helper로 정리

## P101: Release Binary Build Argument Error Smoke

- [x] `target/release/mlg build examples/first.mlg -o`가 missing output diagnostic을 내는지 검증
- [x] `target/release/mlg build examples/first.mlg --wat`가 unknown build argument diagnostic을 내는지 검증
- [x] release build argument failure stdout이 비어 있는지 검증
- [x] 기존 release CLI failure helper를 재사용

## P102: V1 Milestone Roadmap

- [x] `docs/V1_ROADMAP.md`에 `v0.2.0`부터 `v1.0.0`까지 아홉 개 milestone 기록
- [x] 각 milestone의 목표, 범위, 완료 조건, 제외 항목을 구분
- [x] first-class references, interfaces, backend 전환을 automatic v1 scope가 아닌 decision gate로 유지
- [x] README, compiler roadmap, handoff, agent harness에서 장기 roadmap 연결

## P103: v0.2 Project Model Decision Gate

- [x] package/import/visibility, manifest, source layout 선택지와 추천안 기록
- [x] standalone `.mlg` compatibility와 v0.2 제외 범위 기록
- [x] hand-written parser 유지 조건 기록
- [x] 사용자 승인 뒤 language/project surface 확정

## P104: File-Aware Source Model

- [x] token/AST/IR `Span`에 `SourceId` 전파
- [x] 여러 source file과 line/column lookup을 소유하는 `SourceMap` 추가
- [x] 기존 single-file `lex`/`parse` compatibility API 유지
- [x] CLI frontend diagnostic을 file/line/column 형식으로 연결
- [x] multi-file identity, Unicode column, error propagation regression 추가

## P105: Multi-Source Compilation Unit

- [x] `Program`에 원본 파일별 source span 목록 보존
- [x] 여러 `SourceId`의 declaration을 deterministic input order로 합치는
  `parse_sources` 추가
- [x] 기존 single-file CLI parsing을 multi-source frontend entrypoint로 통합
- [x] cross-file function call semantic/C backend regression 추가
- [x] 다른 파일의 parse/duplicate declaration error source identity regression 추가

## P106: Multi-Source Compiler Pipeline

- [x] multi-source `check_sources`, `lower_sources`, `generate_c_sources` API 추가
- [x] frontend, semantic, IR, backend stage를 보존하는 compiler error model 추가
- [x] 기존 single-file CLI check/ir/build/run을 compiler pipeline으로 통합
- [x] cross-file semantic, IR, C backend와 stage/source identity regression 추가

## P107: Explicit Source File Loader

- [x] caller가 제공한 file 순서를 보존하는 `load_source_files` 추가
- [x] loaded `SourceMap`과 ordered `SourceId`를 `SourceSet`으로 전달
- [x] source read failure에 실패 path와 underlying I/O error 보존
- [x] 기존 single-file CLI source loading을 공통 loader로 통합

## P108: Project Manifest and Source Discovery

- [x] strict `mallang.toml` project name parsing 추가
- [x] directory에서 가장 가까운 상위 manifest 탐색
- [x] `src/main.mlg` entry source와 recursive `.mlg` source discovery 추가
- [x] project-relative path 기준 deterministic source order 보장
- [x] direct `.mlg` 입력을 project discovery에서 제외해 standalone 경계 유지

## P109: Package Syntax and File Metadata

- [x] `package`, `import`, `pub` keyword와 parser grammar 추가
- [x] package/import metadata를 source file별 `SourceUnit`으로 보존
- [x] top-level type, function, method에 package-private/public visibility 보존
- [x] package/import 위치와 invalid `pub` declaration diagnostic 추가
- [x] standalone source의 optional package compatibility 유지

## P110: Deterministic Package Graph

- [x] manifest name과 source directory에서 canonical package path 계산
- [x] source directory와 `package` declaration 일치 검증
- [x] package별 type/function/method declaration table 구성
- [x] unresolved/duplicate/conflicting import diagnostic 추가
- [x] 모든 import cycle을 span과 함께 거부하고 dependency-first build order 생성
- [x] 서로 다른 package의 같은 declaration 이름 허용

## P111: Cross-Package Linking and Visibility

- [x] qualified function call, type reference, struct literal parser surface 추가
- [x] package-local declaration을 충돌 없는 internal symbol로 정규화
- [x] imported function/type의 explicit `pub` visibility 검사
- [x] cross-package method call visibility를 semantic receiver resolution에 연결
- [x] public API의 package-private type 노출 거부
- [x] 동명 package declaration과 import qualifier local shadowing 회귀 검증
- [x] linked project를 기존 ownership, IR, C backend까지 전달

## P112: Project Compiler Pipeline

- [x] project용 check/lower/C generation compiler API 추가
- [x] frontend/package/link/semantic/IR/backend error stage 보존
- [x] linked internal symbol을 user-facing package-qualified diagnostic으로 복원
- [x] project source를 기존 ownership, IR, backend pipeline으로 end-to-end 전달

## P113: Project CLI and Native Acceptance

- [x] directory와 `mallang.toml` 입력을 project-aware `check`, `build`, `run`에 연결
- [x] direct `.mlg` 입력의 manifest-free standalone 동작 유지
- [x] project-local 기본 build/run artifact 경로 추가
- [x] 두 package의 function/struct/method native build/run smoke 추가
- [x] project import cycle file/line/column CLI diagnostic smoke 추가
- [x] project generated C strict warning gate 추가

## P114: v0.3 Functions and Closures Decision Gate

- [x] function type과 closure literal 문법 선택지 기록
- [x] owned capture와 escaping closure safety 추천안 기록
- [x] mutable closure call effect와 exclusive access 추천안 기록
- [x] callable C ABI와 cleanup 구현 순서 기록
- [x] 사용자 승인 뒤 v0.3 language surface 확정

## P115: Function Type and Literal Syntax

- [x] `func(T) U`와 `func mut(T) U` function type AST/parser 추가
- [x] function type parameter mode를 type syntax에 보존
- [x] no-value function type의 explicit `unit` return 규칙 추가
- [x] plain/mutable `func(...) { ... }` literal AST/parser 추가
- [x] function literal body를 boxed AST node로 유지해 enum 크기 안정화

## P116: Function Value Semantics and Callable Type Shell

- [x] function type을 semantic `Type`과 signature 비교에 연결
- [x] named function identifier를 fresh move-only function value로 해석
- [x] function parameter, return, local binding의 ownership 검사 추가
- [x] plain/mutable indirect call의 shared/exclusive access와 argument mode 검사 추가
- [x] local value가 동명의 top-level function call을 shadow하도록 일관성 유지
- [x] typed callable C value layout과 cleanup helper shell 추가

## P117: Named Function Value Native Lowering

- [x] typed IR에 named `FunctionValue`와 local `IndirectCall` 추가
- [x] function parameter/return/local move를 cleanup insertion에 연결
- [x] 반환식을 cleanup보다 먼저 평가해 callable use-after-drop 방지
- [x] named function용 environment-free C call thunk 생성
- [x] higher-order parameter/return과 반복 indirect call native smoke 추가
- [x] generated C strict warning과 ASan/UBSan smoke 통과

## P118: Plain Closure Capture Semantics

- [x] lexical scope를 반영한 free-variable capture 수집 추가
- [x] capture 순서, type, function signature를 checked metadata에 보존
- [x] Copy capture는 원본 재사용을 허용하고 non-Copy capture는 생성 시 move
- [x] borrowed non-Copy와 active range source capture 거부
- [x] plain closure capture mutation과 capture value move-out 거부
- [x] nested/mutable function literal을 후속 lowering 전까지 명시적으로 거부

## P119: Owned Closure Environment Native Lowering

- [x] typed IR에 closure definition, capture field, closure value 추가
- [x] capture 있는 closure용 typed heap environment와 allocation failure guard 생성
- [x] environment pointer를 capture local로 해석하는 C call body 생성
- [x] capture type별 cleanup 뒤 environment를 해제하는 drop thunk 생성
- [x] escaping closure의 Copy/slice capture와 반복 indirect call native smoke 추가
- [x] generated C strict warning과 ASan/UBSan cleanup smoke 통과

## P120: Mutable Closure Capture and Native Lowering

- [x] 대입, mutable borrow, mutable callable과 `mut` receiver 사용을 변경 캡처로 분류
- [x] 변경 캡처의 mutable source binding 요구와 plain closure 불변 규칙 고정
- [x] `func mut` effect와 변경 캡처 metadata를 semantic/typed IR에 보존
- [x] closure environment field를 mutable call body lvalue로 연결
- [x] Copy 원본 격리, owned slice 상태, nested callable cleanup native smoke 추가
- [x] immutable source와 immutable callable access rejection 회귀 검증

## P121: Nested Closure Capture Propagation

- [x] nested literal free variable을 enclosing closure capture로 전파
- [x] nested checker가 enclosing parameter/local/capture를 생성 시 다시 copy/move
- [x] borrowed non-Copy outer capture의 nested move 거부
- [x] nested plain/mutable function type과 capture metadata를 typed IR에 보존
- [x] owned slice outer environment와 invocation-local inner capture native smoke 추가
- [x] nested mutable state 독립성과 environment cleanup sanitizer smoke 추가

## P122: Package Function Values and Closure API

- [x] unqualified package-local named function을 value position에서 internal symbol로 연결
- [x] imported `pkg.Function` value를 public function declaration으로 검증해 연결
- [x] private/non-function package selector value diagnostic 유지
- [x] public function type parameter/return의 nested type visibility 검증
- [x] cross-package higher-order parameter, named return, closure return native smoke 추가
- [x] project generated C strict warning과 ASan/UBSan smoke 추가

## P123: v0.3 Closure Safety Acceptance

- [x] borrowed non-Copy capture CLI rejection fixture 추가
- [x] immutable source의 mutable capture CLI rejection fixture 추가
- [x] function value use-after-move와 same-call mutable alias fixture 추가
- [x] recursive closure initializer 전용 source diagnostic 추가
- [x] invalid fixture의 file/line/column CLI diagnostic gate 추가
- [x] full Rust/C/project gate와 56-program generated C sanitizer sweep 통과
- [x] SPEC, v1 roadmap, handoff를 implementation complete/release pending으로 동기화

## P124: v0.4 Generic Data Model Decision Gate

- [x] user-defined enum declaration과 variant qualification 선택지 기록
- [x] generic type/function declaration과 explicit type argument 추천안 기록
- [x] nested pattern과 exhaustiveness의 v0.4 범위 기록
- [x] built-in `Option`/`Result` compatibility migration 경계 기록
- [x] project-wide monomorphization과 generic ownership/cleanup 계약 기록
- [x] generic receiver와 excluded feature 경계 기록
- [x] 사용자 승인 뒤 v0.4 language surface 확정

## P125: Generic Declaration and Pattern Syntax Shell

- [x] `enum` keyword와 `Program.enums` declaration AST 추가
- [x] generic struct/function type parameter declaration parser 추가
- [x] zero/single-payload enum variant parser와 source span 보존
- [x] generic struct literal과 one/multi type argument value application AST 보존
- [x] qualified, nested, wildcard match pattern parser 추가
- [x] multi-source merge와 linker expression traversal에 새 AST 연결
- [x] semantic lowering 전 generic/enum declaration의 명시적 단계 진단 추가
- [x] parser/semantic 회귀 테스트와 full Rust/Clippy gate 통과

## P126: Owned Checked Program Foundation

- [x] checked function/struct symbol table key를 owned string으로 전환
- [x] `CheckedProgram`이 checked AST를 `Arc<Program>`으로 소유하도록 전환
- [x] IR lowerer의 입력 lifetime을 checked program 소유권과 분리
- [x] closure capture collector를 owned struct symbol table에 연결
- [x] 기존 standalone/project compiler API 호환성 유지
- [x] full Rust/Clippy regression gate 통과

## P127: Demand-driven Generic Struct and Function Specialization

- [x] generic declaration을 concrete AST로 변환하는 owned specialization pass 추가
- [x] declaration symbol과 type argument 기반 deterministic key/internal name 생성
- [x] generic struct, function, function value의 explicit type argument specialization 연결
- [x] 동일 key 재사용, 잘못된 arity, expanding specialization cycle 진단 추가
- [x] slice를 포함한 concrete type substitution과 기존 ownership/cleanup 경로 재사용
- [x] standalone generic example의 native output, strict generated C, ASan/UBSan gate 추가

## P128: Symbolic Generic Validation and Receiver Specialization

- [x] 사용 여부와 무관하게 모든 generic struct/function body를 symbolic demand로 검사
- [x] unconstrained type parameter를 non-Copy, non-printable concrete sentinel로 검증
- [x] symbolic internal type name을 source type parameter diagnostic으로 복원
- [x] generic receiver의 declaration type parameter binding과 independent generic 거부
- [x] concrete struct specialization마다 `con`/`mut` receiver method 생성
- [x] non-Copy generic field 교체의 native output, strict C, ASan/UBSan gate 추가

## P129: Package-aware Generic Resolution

- [x] package declaration metadata에 generic arity와 enum type kind 보존
- [x] declaration-scoped type parameter namespace와 local type shadowing 처리
- [x] imported generic struct/function/receiver를 package internal symbol로 연결
- [x] nested imported generic type argument와 value index expression 구분
- [x] public generic API와 enum payload의 private type 노출 거부
- [x] cross-package generic native output, strict C, ASan/UBSan gate 추가

## P130: Generic Enum Specialization and Constructor Semantics

- [x] generic/non-generic enum constructor를 concrete `EnumConstructor` AST로 정규화
- [x] declaration/type argument key 기반 generic enum specialization과 동일 key 재사용
- [x] concrete enum/variant signature, payload type, constructor arity 검사 추가
- [x] empty/duplicate variant와 recursive enum value type source diagnostic 추가
- [x] imported public generic enum constructor와 private visibility 경계 연결
- [x] concrete specialization internal name을 source generic 표기로 진단 복원
- [x] full Rust test와 Clippy regression gate 통과; IR/C lowering은 P131 이후로 유지

## P131: Nested User Enum Pattern Semantics

- [x] specialized enum에 source/package pattern origin metadata 보존
- [x] local/imported `Enum.Variant` pattern qualifier를 package internal symbol로 연결
- [x] user enum과 nested user enum/`Option`/`Result` payload pattern type 검사
- [x] finite variant path coverage 기반 recursive exhaustiveness 검사 추가
- [x] wildcard, duplicate/unreachable arm, payload arity/type mismatch diagnostic 추가
- [x] expression/statement match binding scope와 cross-package generic enum 회귀 검증
- [x] full Rust test와 Clippy regression gate 통과; IR/C pattern lowering은 다음 단계로 유지

## P132: User Enum Typed IR

- [x] specialized enum의 concrete variant 이름과 payload type을 typed IR에 보존
- [x] user enum constructor를 typed payload expression과 함께 IR로 lowering
- [x] user enum과 nested built-in payload pattern을 recursive IR pattern tree로 lowering
- [x] wildcard payload를 cleanup이 필요한 경우 내부 owned binding으로 정규화
- [x] expression/statement match arm-local payload move와 cleanup insertion 연결
- [x] typed IR 구조와 non-Copy wildcard payload cleanup 회귀 테스트 추가
- [x] C backend가 새 enum IR을 지원하기 전 명시적 invariant error 경계 유지

## P133: User Enum Native C Backend

- [x] specialized enum마다 concrete tag와 payload union C layout 생성
- [x] active variant tag에 따라 non-Copy payload를 정리하는 recursive drop helper 생성
- [x] zero/single-payload user enum constructor를 designated initializer로 lowering
- [x] expression/statement match가 공유하는 recursive pattern condition/binding planner 추가
- [x] nested user enum과 `Option`/`Result` payload의 short-circuit tag 검사 연결
- [x] malformed runtime tag trap과 malformed enum constructor IR 회귀 검사 추가
- [x] generic enum, nested pattern, wildcard slice cleanup native example 추가
- [x] native output, full generated C warning-clean, ASan/UBSan gate 연결

## P134: Generic Enum Package and Diagnostic Acceptance

- [x] public generic enum declaration을 imported package metadata와 specialization에 연결
- [x] imported generic enum constructor와 package-qualified pattern native smoke 추가
- [x] cross-package owned slice payload wildcard cleanup을 sanitizer gate에 연결
- [x] nested non-exhaustive path와 constructor payload mismatch CLI fixture 추가
- [x] invalid fixture의 source file/line/column과 source generic spelling 검증
- [x] project generated C warning-clean 및 ASan/UBSan acceptance 통과

## P135: Built-in ADT Common Path

- [x] `Option`/`Result`와 user enum을 semantic ADT metadata view로 정규화
- [x] built-in source pattern spelling을 공통 payload 검사와 finite coverage에 연결
- [x] constructor를 공통 `VariantConstructor`, pattern을 recursive `Variant` IR로 통합
- [x] tag/payload union, constructor, match와 cleanup C lowering을 공통 backend 경로로 통합
- [x] 기존 `Some`/`None`/`Ok`/`Err` source syntax와 native print output 유지
- [x] top-level wildcard와 nested built-in pattern 공통 IR 회귀 테스트 추가
- [x] legacy built-in 전용 IR node, match emitter와 payload field 경로 제거

## P136: v0.4 Generic Data Model Closeout

- [x] v0.4 완료 조건을 unit, CLI diagnostic, native output과 sanitizer gate에 매핑
- [x] user generic type/function/receiver와 generic enum concrete specialization 재검증
- [x] nested exhaustive match, invalid constructor/pattern과 source diagnostic 재검증
- [x] non-Copy payload cleanup과 multi-package visibility acceptance 재검증
- [x] interface/trait는 현재 use case에 필요하지 않아 decision-gated 제외 유지
- [x] `scripts/check.sh` canonical gate와 publication boundary gate 통과
- [x] `docs/V1_ROADMAP.md`를 implementation complete, release pending으로 갱신

## P137: v0.5 Ownership and Runtime Decision Gate

- [x] current Copy/move/drop, heap allocation과 cleanup path inventory 작성
- [x] user-visible `Box`/`Heap` 없이 transparent recursive ADT 방향 확정
- [x] positional multi-payload enum과 compiler-owned recursive representation 확정
- [x] general partial move/`replace` 제외와 temporary cleanup 경계 작성
- [x] first-class reference, range borrow와 fatal runtime failure 추천안 작성
- [x] memory-safety acceptance와 implementation order 초안 작성
- [x] 사용자 승인 뒤 v0.5 language/runtime contract 확정

## P138: Positional Multi-Payload Enum Surface

- [x] enum variant declaration을 zero/one/multiple payload list로 일반화
- [x] constructor argument arity와 payload type source diagnostic 추가
- [x] pattern payload list, wildcard와 nested pattern parse/semantic 연결
- [x] existing zero/single payload source compatibility regression 유지
- [x] specialized generic/imported enum metadata와 linker 경로 일반화
- [x] typed IR/backend 전 explicit invariant boundary 유지

## P139: Recursive Type Graph Validation

- [x] concrete struct/user enum payload와 nested wrapper를 포함한 type dependency graph 작성
- [x] recursive SCC가 user enum과 non-recursive base variant를 모두 가질 때만 허용
- [x] direct/mutual struct-only recursion과 base 없는 enum recursion source diagnostic 유지
- [x] built-in `Option`/`Result`만으로 생긴 cycle에는 implicit indirection을 부여하지 않음
- [x] generic specialization과 imported enum의 recursive graph를 concrete type 기준으로 검사
- [x] accepted recursive enum과 rejected cycle shape semantic regression 추가
- [x] indirect representation 구현 전 recursive enum typed IR invariant boundary 유지

## P140: Recursive Multi-Payload Typed IR

- [x] `IrEnumVariant`와 recursive `IrMatchPattern`을 positional payload list로 일반화
- [x] recursive enum을 non-recursive inline enum과 구분하는 typed storage metadata 추가
- [x] constructor argument를 left-to-right로 평가한 뒤 owned payload slot으로 이동
- [x] consuming match에서 active payload 전체 binding/wildcard와 storage shell release 표현
- [x] recursive/multi-payload cleanup binding과 drop path를 typed IR에 보존
- [x] non-recursive zero/single payload IR compatibility regression 유지
- [x] recursive generic enum constructor/match typed IR acceptance 추가

## P141: Multi-Payload and Recursive Enum C Runtime

- [x] inline multi-payload variant의 C payload struct와 tagged union layout 생성
- [x] recursive enum의 compiler-owned node/handle layout과 forward declaration 생성
- [x] constructor payload를 left-to-right temporary로 평가하고 allocation failure guard 연결
- [x] consuming match가 active payload 전체를 move/bind한 뒤 owned storage shell을 한 번 해제
- [x] active variant payload를 순회하는 recursive drop helper와 malformed handle guard 생성
- [x] non-recursive zero/single payload C ABI와 native output compatibility 유지
- [x] generic recursive enum의 constructor/match/drop native 및 ASan/UBSan acceptance 추가

## P142: Full-Expression Temporary Cleanup

- [x] cleanup value temporary를 typed IR의 full-expression scope로 모델링
- [x] call argument와 discarded expression temporary를 statement 종료 시 정확히 한 번 정리
- [x] `if`/`for` condition temporary를 각 평가 직후 정리하고 short-circuit 순서 유지
- [x] index/`len`/range source temporary cleanup과 bounds guard 순서 연결
- [x] return, `break`/`continue`와 runtime failure 경로의 temporary ownership 계약 고정
- [x] 기존 inline slice index/`len`/range 제한을 안전한 temporary cleanup 경로로 교체
- [x] strict C, native output와 ASan/UBSan temporary-heavy acceptance 추가

## P143: Static and Owned String Runtime

- [x] current static string literal ABI, move rule와 cleanup gap inventory 작성
- [x] static/owned storage를 같은 immutable `string` value로 표현하는 typed IR/C contract 고정
- [x] static literal은 해제하지 않고 owned buffer는 정확히 한 번 해제하는 drop helper 구현
- [x] string parameter/return/local/field/enum/closure ownership을 공통 cleanup 경로에 연결
- [x] print/equality가 storage kind와 무관하게 같은 value semantics를 유지
- [x] malformed owned string과 allocation failure의 fatal no-unwind invariant 추가
- [x] strict C, native output와 ASan/UBSan string ownership acceptance 추가

## P144: Borrow and Range Exclusion Contract

- [x] `con`/`mut`가 direct call-scoped mode이고 first-class reference가 아님을 regression으로 고정
- [x] borrowed non-Copy move/return/store/capture와 overlapping mutable access 진단 matrix 보강
- [x] by-reference/mutable range binding syntax가 reserved diagnostic으로 거부되는지 고정
- [x] non-Copy range의 index-only traversal과 indexed `con`/`mut` access acceptance 유지
- [x] use-after-move, overwrite, return, branch와 loop ownership merge 규칙을 `SPEC.md`와 동기화
- [x] accepted/rejected borrow-range fixture를 CLI diagnostic 및 native gate에 연결
- [x] strict C와 generated C sanitizer sweep에서 기존 ownership runtime 회귀 없음 확인

## P145: Allocation Accounting and Failure Injection

- [x] slice, closure, recursive enum과 owned string allocation/free path inventory 작성
- [x] compiler runtime allocation을 공통 accounting 가능한 helper contract로 연결
- [x] source surface에 노출하지 않는 deterministic allocation failure injection 경로 추가
- [x] normal exit에서 allocation/free count가 일치하는 cleanup-heavy native harness 추가
- [x] allocation size overflow와 injected failure의 stable fatal no-unwind diagnostic 고정
- [x] return, branch, loop, overwrite와 nested aggregate cleanup accounting regression 추가
- [x] strict C, full project gate와 generated C ASan/UBSan sweep 통과
- [x] v0.5 completion evidence와 v0.6 decision gate 문서 동기화

## P146: v0.6 Standard Library Decision Gate

- [x] current package/linker/semantic/IR/backend feasibility와 roadmap scope gap inventory 작성
- [x] standard package namespace, resolution과 runtime/compiler ownership 추천안 승인
- [x] process arguments/environment와 `main` signature 경계 승인
- [x] UTF-8 string operation, byte index와 allocation semantics 승인
- [x] file/stream I/O, standard `Error`와 `Result` API surface 승인
- [x] error propagation syntax는 v0.6에서 제외하고 P152에서 재평가하기로 승인
- [x] owned key-value collection type, key restriction과 mutation API 승인
- [x] platform support와 standard-library native acceptance matrix 승인
- [x] Q1-Q8 승인 결정을 P147-P153 implementation order와 compatibility contract로 확정

## P147: Standard Package Registry and Intrinsic ABI

- [x] reserved `std/...` package registry와 exact import resolution 추가
- [x] project/standalone compilation을 shared standard-aware linking path로 연결
- [x] standard public type/function signature와 explicit generic specialization 연결
- [x] standard call target과 function value를 typed intrinsic identity로 semantic/IR에 보존
- [x] opaque `Map[K,V]`, supported key type와 direct construction restriction을 semantic에 보존
- [x] unknown standard package, shadow, wrong arity/mode/type와 internal-name access 진단 추가
- [x] project/standalone CLI check와 IR acceptance, existing project compatibility 검증
- [x] runtime body가 없는 intrinsic call의 deterministic backend invariant diagnostic 고정

## P148: UTF-8 Text and Standard Error

- [x] `errors.Kind`/`errors.Error` native representation과 platform-independent category mapping 추가
- [x] string byte/scalar count, contains/find와 UTF-8 validation runtime 구현
- [x] split/join과 int/bool conversion/parse intrinsic 구현
- [x] owned string/slice/error result를 allocation accounting와 cleanup에 연결
- [x] invalid UTF-8, parse overflow와 empty separator semantics regression 추가
- [x] strict C, sanitizer와 allocation failure injection acceptance 통과

## P149: Process and Stream I/O

- [x] generated C `main` process ABI와 demand-driven `std/os` runtime 연결
- [x] UTF-8 검증과 owned cleanup을 적용한 `os.args`, `os.env`, `os.exit` 구현
- [x] `mlg run --` argument forwarding과 numeric exit status parity 고정
- [x] `io.readStdin`, `io.writeStdout`, `io.writeStderr` recoverable runtime 구현
- [x] invalid UTF-8, embedded NUL, missing env와 closed stream failure 검증
- [x] direct/runner invocation, strict C, allocation accounting/failure injection 검증
- [x] normal/error process path ASan/UBSan acceptance 통과

## P150: File I/O

- [x] demand-driven `fs.readText`/`fs.writeText` runtime과 callable thunk 연결
- [x] NUL-free path 변환, UTF-8 read와 embedded NUL content 보존
- [x] create-or-overwrite exact write와 short-write detection 구현
- [x] open/read/write/close failure를 platform-independent `errors.Kind`로 mapping
- [x] NotFound, PermissionDenied, InvalidInput과 InvalidData native regression 추가
- [x] strict C, zero-allocation accounting과 deterministic failure injection 검증
- [x] success/error file path ASan/UBSan acceptance 통과

## P151: Owned Map

- [x] specialized opaque `Map[K,V]` handle과 separately allocated entry node layout 구현
- [x] deterministic `int`/`bool`/UTF-8 string hash/equality와 bucket growth 구현
- [x] `newMap`, `count`, `insert`, `with`, `update`, `remove` typed runtime 연결
- [x] direct call과 concrete generic standard function-value thunk 이름 충돌 방지
- [x] replacement key cleanup, old value return, removal ownership transfer와 remaining-entry drop 구현
- [x] Copy/non-Copy key/value, 24-entry growth와 callback read/update native regression 추가
- [x] strict C, zero-allocation accounting, deterministic failure injection과 ASan/UBSan 통과
- [x] 전체 526개 unit test와 67-program generated C sanitizer sweep 통과

## P152: Reference CLI and Error Flow Review

- [x] `examples/projects/textstats` multi-module native CLI 추가
- [x] input/output arguments, UTF-8 file read와 file/stdout write workflow 연결
- [x] `Map[int,int]` line-length histogram 기반 text summary transformation 구현
- [x] expected `Result` failure를 stderr와 stable non-zero exit로 변환
- [x] stdout/output-file, usage, missing/invalid input와 write failure regression 추가
- [x] strict C, zero-allocation accounting과 ASan/UBSan acceptance 통과
- [x] 5개 `Result` match, 10개 arm, 3-level nesting evidence 기록
- [x] `?`는 v0.6에 추가하지 않고 additional evidence 뒤 재검토하기로 판정

## P153: v0.6 Acceptance and Documentation

상태: complete

- [x] `docs/STANDARD_LIBRARY.md` public API/ownership/failure reference 추가
- [x] `SPEC.md`, README, roadmap와 handoff를 P151-P152 implementation에 동기화
- [x] reference CLI와 standard runtime을 optimized release compiler smoke에 연결
- [x] local macOS arm64 canonical/release/strict-C/ASan/UBSan acceptance 통과
- [x] Ubuntu `ubuntu-latest`가 같은 canonical `scripts/check.sh`를 실행하도록 CI 연결 확인
- [x] published `main`의 Ubuntu Linux x86_64 CI success 확인
- [x] v0.6 completion evidence와 P154-P160 v0.7 decision gate 초안 작성

P153 complete: local macOS arm64와 published Ubuntu Linux x86_64 acceptance가 모두 통과했고,
v0.6.0 GitHub source release가 2026-07-15에 공개됐다.

## P154: v0.7 Tooling Decision Gate

상태: complete

- [x] formatter trivia/comment preservation gap inventory
- [x] project test discovery/runner gap inventory
- [x] local path dependency graph gap inventory
- [x] structured JSON diagnostic schema feasibility
- [x] macOS arm64/Linux x86_64 artifact/install feasibility
- [x] basic LSP release-blocker assessment
- [x] Q1-Q6 compatibility recommendation approval

P154 inventory와 Q1-Q6 결정은 `docs/todo-v07-tooling-platforms/feasibility.md` 및
`open-questions.md`가 소유한다. 추천안은 2026-07-15 승인됐다.

## P155: Canonical Formatter

상태: complete

- [x] parser validation 뒤 raw token span과 `//` trivia를 보존하는 formatter 추가
- [x] 4-space indent, LF, final newline, blank line 최대 1개 canonical style 고정
- [x] direct `.mlg`와 deterministic project source formatting 구현
- [x] `mlg fmt --check <input>` no-write/non-zero contract 구현
- [x] project parse failure 시 어떤 source도 쓰지 않는 preflight contract 검증
- [x] non-trivia token/comment parity와 checked-in example idempotence regression 추가
- [x] debug canonical gate와 optimized release binary smoke 연결

P155는 source meaning과 comment text를 보존하며 line-width wrapping과 block comment를
의도적으로 제외한다. 다음 milestone은 P156 project test workflow다.

## P156: Project Test Workflow

상태: complete

- [x] parser/project/package/compiler/native execution gap inventory
- [x] optional `tests/` recursive deterministic discovery API와 regressions
- [x] contextual declaration/assertion, package mapping, process isolation 추천안 작성
- [x] stable test ID/order/exact filter와 output/exit contract 추천안 작성
- [x] ownership/native/sanitizer acceptance matrix 작성
- [x] P156 Q1-Q6 recommendation approval
- [x] parser, linker, semantic, IR/backend와 `mlg test` implementation
- [x] deterministic test ID/order, exact filter, output aggregation와 exit contract smoke
- [x] zero-allocation, strict C, ASan/UBSan와 debug/release CLI acceptance

P156의 exact contract는
`docs/todo-v07-tooling-platforms/p156-test-workflow.md`가 소유한다. Contextual
test/assert surface와 test별 synthetic native process contract를 완료했다. 다음 milestone은
P157 local path dependencies다.

## P157: Local Path Dependencies

상태: complete

- [x] current manifest/project/package loading gap inventory
- [x] exact relative path, graph/import와 library command contract
- [x] recursive dependency-first project discovery와 canonical deduplication
- [x] cross-project package identity, direct dependency import와 visibility linking
- [x] library check/test 및 executable build/run entrypoint boundary
- [x] multi-project native, strict C, allocation, sanitizer와 debug/release CLI acceptance

P157의 exact contract는
`docs/todo-v07-tooling-platforms/p157-local-path-dependencies.md`가 소유한다.
다음 milestone은 P158 machine-readable diagnostics다.

## P158: Machine-readable Diagnostics

상태: complete

- [x] versioned `mallang.diagnostic.v1` model과 stable stage vocabulary
- [x] shared human/JSON renderer와 existing human diagnostic parity
- [x] global `--diagnostic-format <human|json>` CLI contract
- [x] UTF-8 byte span, Unicode scalar location과 project/dependency path normalization
- [x] CLI/input/frontend/package/link/semantic/native JSONL binary matrix
- [x] formatter multi-record, failed test assertion와 successful stdout contract
- [x] standard-library-only JSONL consumer와 debug/release smoke
- [x] basic LSP를 v0.7 blocker에서 제외하고 P160 decision gate로 보류

P158의 exact contract는
`docs/todo-v07-tooling-platforms/p158-machine-readable-diagnostics.md`가 소유한다.
다음 milestone은 P159 release artifacts and installation이다.

## P159: Release Artifacts and Installation

상태: complete

- [x] `MIT OR Apache-2.0` package metadata와 archive license payload
- [x] macOS arm64/Linux x86_64 host detection과 exact archive naming
- [x] normalized tar/gzip metadata와 repeated-build byte identity
- [x] one-target local 및 exact two-target release `SHA256SUMS` writer
- [x] explicit-version HTTPS/offline installer와 default/explicit prefix
- [x] checksum, archive entry set와 staged `mlg --version` verification
- [x] atomic install/reinstall 및 installed project check/build/run/test smoke
- [x] pinned GitHub Actions native matrix와 combined archive/checksum/installer bundle
- [x] canonical local gate와 public docs synchronization
- [x] published macOS arm64/Linux x86_64 jobs와 combined bundle download/checksum 확인

P159의 exact contract는
`docs/todo-v07-tooling-platforms/p159-release-artifacts-installation.md`가 소유한다. Local
implementation과 published native matrix evidence를 모두 확인했다. 다음 milestone은 P160 v0.7
acceptance다.

## P160: v0.7 Acceptance

상태: complete; released as v0.7.0 (2026-07-16)

- [x] 빈 work directory에 library와 dependent executable project 생성
- [x] installed release compiler의 formatter no-write/idempotence 검증
- [x] human/JSON check, project test, native build/run canonical workflow
- [x] canonical local gate와 macOS arm64/Linux x86_64 release matrix 연결
- [x] README, `SPEC.md`, handoff와 v0.8 decision draft 동기화
- [x] local canonical/publication gate 통과
- [x] published platform matrix와 combined bundle evidence
- [x] v0.8 Q1-Q6 사용자 승인

P160의 exact workflow는
`docs/todo-v07-tooling-platforms/p160-v07-acceptance.md`가 소유한다. v0.7.0은 두 supported
native archive, checksum과 installer를 포함한 GitHub Release로 공개됐다. 다음 milestone은
approved v0.8 hardening의 P161 baseline inventory다.

## P161: v0.8 Hardening Baseline Inventory

상태: complete (2026-07-16)

- [x] fail-fast lexer/parser/multi-source/compiler diagnostic flow inventory
- [x] CLI multi-record rendering reuse boundary 확인
- [x] production panic/invariant audit classification 정의
- [x] deterministic property와 crash-corpus gap 기록
- [x] standalone/dependency/reference CLI performance baseline set 고정
- [x] generated C/release archive reproducibility gap 기록
- [x] P162 top-level recovery, block recovery와 cap acceptance slice 분리

P161의 current-source evidence와 P162 exact slice order는
`docs/todo-v08-compiler-hardening/p161-baseline-inventory.md`가 소유한다.

## P162: Parser Recovery and Multiple Diagnostics

상태: complete (2026-07-16)

- [x] 기존 single-error convenience API 보존
- [x] top-level recovery와 source별 최대 32개 parse diagnostic
- [x] deterministic multi-source frontend/compiler aggregation
- [x] CLI `parse/check/ir/build/run/test` multi-record 연결
- [x] human/JSON parity와 semantic-stage 차단 회귀
- [x] delimiter-aware block statement recovery
- [x] nested function literal, unclosed block와 receiver method ambiguity 회귀
- [x] exact duplicate suppression과 stable span order
- [x] first-32 truncation과 lexical fail-fast acceptance
- [x] `parse/check/ir/build/run/test` human/JSON/non-zero compatibility

Slice A의 API, recovery boundary와 검증 증거는
`docs/todo-v08-compiler-hardening/p162-parser-recovery.md`가 소유한다. 이어지는 P163에서
user-reachable panic/invariant와 malformed typed IR 방어를 분류하고 제거했다.

## P163: Compiler and IR Invariant Defense

상태: complete (2026-07-16)

- [x] production panic/expect/unchecked-index site를 세 범주로 재분류
- [x] direct parser token input의 EOF sentinel 자체 보장
- [x] match pattern/receiver span의 user-adjacent `expect`/`unwrap` 제거
- [x] empty match arm을 semantic/IR diagnostic으로 전환
- [x] frontend/package/semantic malformed-source stage regression
- [x] backend declaration preflight validator
- [x] duplicate declaration/field와 invalid `main` typed IR negative tests
- [x] 기존 backend local invariant negative matrix 보존

분류와 validator 경계는
`docs/todo-v08-compiler-hardening/p163-invariant-defense.md`가 소유한다. 다음 milestone은
deterministic mutation property와 minimized crash corpus를 추가하는 P164다.

## P164: Property and Crash-corpus Testing

상태: complete (2026-07-16)

- [x] 256-seed deterministic arbitrary UTF-8 lexer property
- [x] token delete/duplicate/five-kind replacement parser mutation property
- [x] type/ownership five-case known-invalid transformation property
- [x] frontend/package/link/semantic/ownership six-file minimized corpus
- [x] corpus file registration completeness guard
- [x] stable toolchain Cargo integration gate

Generator, corpus promotion과 canonical gate는
`docs/todo-v08-compiler-hardening/p164-property-crash-corpus.md`가 소유한다. 다음 milestone은
representative 성능 측정과 same-input output identity를 고정하는 P165다.

## P165: v0.8 Performance and Reproducibility Baseline

상태: complete (2026-07-16)

- [x] 네 representative case의 release-profile repeated measurement harness
- [x] check/build/runtime median과 generated C/native size machine-readable record
- [x] observational policy와 unset regression threshold schema
- [x] runtime output 및 generated C SHA-256 기록
- [x] generated C same-input byte identity gate
- [x] existing release archive byte identity gate composition
- [x] native executable byte identity 제외 범위 고정

측정 schema, initial observation과 reproducibility 범위는
`docs/todo-v08-compiler-hardening/p165-performance-reproducibility.md`가 소유한다. 다음
milestone은 full hardening evidence와 v0.9 freeze decision gate를 닫는 P166이다.

## P166: v0.8 Compiler Hardening Acceptance

상태: complete; released as v0.8.0 (2026-07-16)

- [x] debug/release CLI crash-corpus stage/message parity
- [x] parser recovery, full examples, warning-clean C와 focused sanitizer gate
- [x] complete generated C ASan/UBSan native-output identity
- [x] generated C와 deterministic release archive byte identity
- [x] installed optimized compiler clean-project workflow
- [x] macOS arm64/Linux x86_64 CI release artifact와 checksum bundle
- [x] package version, published spec와 v0.8 release notes
- [x] observational performance threshold second decision
- [x] v0.9 language-freeze Q1-Q6와 P167-P172 implementation order

Exact command composition과 platform evidence boundary는
`docs/todo-v08-compiler-hardening/p166-v08-acceptance.md`가 소유한다. 다음 milestone은
frozen v1 candidate의 normative rule inventory를 만드는 P167이다.

## P167: v1 Candidate Normative Contract Inventory

상태: complete (2026-07-16)

- [x] source, lexical, project, type, function, control-flow와 ownership rule ID
- [x] standard library exact API의 normative detail-owner 연결
- [x] stable CLI와 inspection command output stability 경계
- [x] diagnostic, supported target, artifact와 runtime rule ID
- [x] stale Copy/move, user enum, nested match wording 교정
- [x] P168 compatibility, P169 conformance/migration, P170 dogfood blocker inventory

Candidate contract와 current-source drift audit은
`docs/V1_LANGUAGE_CONTRACT.md`와
`docs/todo-v09-language-freeze/p167-normative-inventory.md`가 소유한다. 다음 milestone은
compiler/language version과 v1 compatibility 약속을 고정하는 P168이다.

## P168: Version and Compatibility Policy

상태: complete (2026-07-17)

- [x] compiler release와 implemented language contract의 단일 version model
- [x] `v0.9.0` candidate freeze와 `v1.0.0` first stable 관계
- [x] v1.x source acceptance와 observable semantics guarantee
- [x] patch/minor/major change classification
- [x] deprecation notice와 next-major removal policy
- [x] narrow soundness/security compatibility exception
- [x] edition, manifest version field와 source pragma 제외
- [x] stable surface와 implementation detail 경계

공개 정책과 normative rule은 `docs/COMPATIBILITY.md`,
`docs/V1_LANGUAGE_CONTRACT.md`의 `V1-COMP-001`-`013`, 그리고
`docs/todo-v09-language-freeze/p168-version-compatibility.md`가 소유한다. 다음 milestone은
98개 rule을 evidence와 연결하고 0.x migration을 통합하는 P169다.

## P169: Conformance and Migration Map

상태: complete (2026-07-17)

- [x] 98개 contract rule의 exact-set evidence manifest
- [x] 23개 evidence profile과 64개 script/fixture/Rust-test/command item
- [x] duplicate, unmapped, unknown rule과 stale evidence fail-closed checker
- [x] canonical `scripts/check.sh` conformance integration
- [x] bootstrap borrow syntax와 0.x project/ownership/standard/tooling migration guide
- [x] canonical borrow/range check-build-run fixture
- [x] suffix `in`/`mut`, call `in`, by-reference range rejection fixtures

Conformance schema, checker, migration guide와 executable acceptance는
`docs/conformance/v1-rules.json`, `scripts/check-v1-conformance.py`,
`docs/MIGRATION_V1.md`, `scripts/check-v1-migration.sh`, 그리고
`docs/todo-v09-language-freeze/p169-conformance-migration.md`가 소유한다. 다음 milestone은
representative `textstats`를 clean workflow로 반복 검증하는 P170이다.

## P170: Representative Dogfood

상태: complete (2026-07-17)

- [x] deterministic release archive의 clean-prefix installed compiler
- [x] ignored output을 제외한 clean `textstats` project copy
- [x] formatter no-write/idempotence와 canonical source normalization
- [x] UTF-8 summary package test와 deterministic test discovery/output
- [x] representative project와 dedicated empty-suite test fixture 분리
- [x] format/check/test/build/run 두 번 반복
- [x] stdout, output-file, usage exit와 generated C identity
- [x] strict C, ASan/UBSan와 allocation-accounting reference CLI gate
- [x] compiler, diagnostic, documentation, test-gap 분류와 frozen-surface no-change 판정

Standalone clean-install gate와 issue inventory는
`scripts/check-v09-dogfood.sh`와
`docs/todo-v09-language-freeze/p170-representative-dogfood.md`가 소유한다. 다음 milestone은
freeze audit, supported-platform artifacts와 `v0.9.0` release를 닫는 P171이다.

## P171: v0.9 Acceptance and Release

상태: complete; released as v0.9.0 (2026-07-17)

- [x] signed `v0.8.0` base 이후 compiler source zero-change audit
- [x] documentation, conformance, dogfood와 release change classification
- [x] 98 rules, 23 profiles와 64 evidence item completeness
- [x] canonical, optimized release compiler와 complete generated C sanitizer gate
- [x] macOS arm64/Linux x86_64 target archive와 checksum bundle
- [x] package version, published v0.9 spec와 release notes
- [x] signed `v0.9.0` tag와 public GitHub binary release

Exact freeze audit, acceptance composition과 platform evidence boundary는
`scripts/check-v09-freeze.py`, `scripts/check-v09-acceptance.sh`와
`docs/todo-v09-language-freeze/p171-v09-acceptance.md`가 소유한다. 다음 milestone은
v1 RC clean install, v0.9 upgrade와 rollback rehearsal를 수행하는 P172다.

## P172: v1 RC and Rollback Rehearsal

상태: complete; released as v1.0.0-rc.1 prerelease (2026-07-17)

- [x] SemVer prerelease archive, checksum, installer와 version identity
- [x] malformed prerelease rejection before build/download
- [x] clean `v1.0.0-rc.1` install and representative project
- [x] same-prefix published v0.9.0 to RC upgrade
- [x] explicit RC to v0.9.0 rollback and RC re-upgrade
- [x] cross-version `textstats` observable-output identity
- [x] canonical, optimized and complete generated C sanitizer gate
- [x] macOS arm64/Linux x86_64 target archive와 checksum bundle
- [x] signed `v1.0.0-rc.1` tag와 public GitHub prerelease

Exact prerelease distribution and rollback sequence는
`scripts/check-v1-rc-rehearsal.sh`, `scripts/check-v1-rc-acceptance.sh`와
`docs/todo-v09-language-freeze/p172-v1-rc-rehearsal.md`가 소유한다. 다음 milestone은
frozen contract를 변경하지 않고 final audit와 `v1.0.0` stable release를 닫는다.

## v1.0.0: Stable Release

상태: complete; released as v1.0.0 (2026-07-17)

- [x] Cargo, compiler, archive, checksum과 installer exact stable version
- [x] v0.9.0 이후 compiler source와 conformance map 불변
- [x] 98 rules / 23 profiles / 64 evidence final completeness
- [x] published RC to stable same-prefix upgrade
- [x] explicit stable to RC rollback and stable re-upgrade
- [x] cross-version `textstats` observable-output identity
- [x] canonical, optimized and complete generated C sanitizer gate
- [x] macOS arm64/Linux x86_64 stable archive와 checksum bundle
- [x] signed `v1.0.0` tag, public stable release와 security reporting boundary

Stable final audit와 배포 sequence는 `scripts/check-v1-stable-acceptance.sh`,
`scripts/check-v1-stable-rehearsal.sh`와
`docs/todo-v1-stable-release/acceptance.md`가 소유한다. 이후 변경은
`docs/COMPATIBILITY.md`의 1.x contract를 따른다.

## P173: v1.1 Streaming Text I/O

상태: complete; released as v1.1.0 (2026-07-17)

- [x] v1.0 compatibility classification과 handle-based API rejection
- [x] generic `fs.forEachLine[C,S]` source contract
- [x] bounded-memory generated C runtime and typed intrinsic specialization
- [x] UTF-8, embedded NUL, LF/CRLF, empty/final-line semantics
- [x] recoverable open/read/close failures
- [x] strict C, ASan/UBSan and allocation-accounting gate
- [x] published v1.0.0 upgrade, rollback, re-upgrade compatibility rehearsal
- [x] supported-platform release artifact acceptance
- [x] signed `v1.1.0` tag and public GitHub release

Exact API decision and evidence are owned by
`docs/todo-v11-streaming-text-io/`. This is a backward-compatible minor release;
it adds no syntax, source-visible handle, borrowed return, or v1.0 semantic
change.

## P174: B0 Self-Hosting Bootstrap Contract

상태: complete (2026-07-17)

- [x] Stage0/Stage1/Stage2와 trusted-seed 경계 정의
- [x] generated C와 conformance fixed-point 판정 정의
- [x] temporary host driver와 compiler-owned semantics 경계 정의
- [x] Rust Stage0으로 tracked Mallang bootstrap probe format/check/test/build
- [x] independent probe build의 generated C byte identity
- [x] exact native probe output과 canonical repository gate
- [x] B1 frontend differential slice와 no-premature-language-change 경계

Exact bootstrap contract and current acceptance are owned by
`docs/SELF_HOSTING.md` and `docs/todo-self-hosting-bootstrap/`. B1 subsequently
closed the complete frontend differential contract.

## P175: B1 Self-Hosting Frontend

상태: complete (2026-07-17)

- [x] UTF-8 byte cursor blocker와 1.x compatibility 분류
- [x] `strings.byteAt`과 scalar-boundary 검증 `strings.slice`
- [x] strict C, ASan/UBSan, allocation accounting과 failure injection gate
- [x] Mallang source/span/token model과 normalized differential schema
- [x] frozen v1 complete lexer와 Rust Stage0 token/diagnostic differential
- [x] flat syntax arena와 declaration/type parser differential
- [x] core statement, Pratt/postfix expression과 literal/call/assignment differential
- [x] statement control flow, test assertion과 match pattern differential
- [x] function literal, if/match expression과 recursive pattern differential
- [x] syntax-only AST와 frozen v1 success-path complete parser
- [x] bounded statement/top-level recovery와 32-error cap differential
- [x] 155-source positive/rejection/crash corpus AST/diagnostic differential
- [x] B1 canonical, publication과 supported-platform CI acceptance

Exact scope and decisions are owned by `docs/todo-self-hosting-frontend/`.
P175a adds only owned standard-library operations and no pointer, borrowed
substring, mutable string or syntax change. P175b adds the tracked Mallang
compiler source root, complete lexer and deterministic Rust differential gate.
P175c1 adds a syntax arena and declaration/type parser while preserving the
existing ownership model. P175c2a adds core statements, Pratt/postfix
expressions and construction/call forms. P175c2b adds statement control flow,
test assertions, function literals, if/match expressions and recursive
patterns. P175c3 adds bounded statement/top-level recovery and the Rust Stage0
diagnostic cap. P175d closes B1 with 155 discovered repository
sources under Stage0, generated Stage1, strict-accounting and sanitizer parity;
the next self-hosting stage is B2 semantic checking and typed IR.

## P176: B2 Self-Hosting Semantics And Typed IR

상태: complete (2026-07-19)

- [x] B2 checker/AST ownership boundary와 stable semantic normalization 계약
- [x] 비제네릭 struct, enum, function, method declaration collection
- [x] primitive, Option, Result, array, slice, nominal, function type resolution
- [x] focused success/rejection Rust Stage0 differential fixtures
- [x] primitive expression, binding/assignment/return checking과 typed IR subset
- [x] direct call, named function value, argument mode와 indirect call typed IR
- [x] field/index read type checking과 typed IR
- [x] mutable field/index assignment place와 typed IR
- [x] nested lexical scope와 if-statement return convergence
- [x] if-expression branch type convergence
- [x] non-Copy local move와 direct local `con`/`mut` call borrow
- [x] nested field/index borrow place와 same-call overlap
- [x] statement/expression `if` branch ownership state join
- [x] condition/conditionless loop persistent ownership state와 loop control
- [x] three-clause init/condition/direct post persistent ownership state
- [x] field/index for-post assignment place
- [x] range loop binding과 persistent ownership state join
- [x] direct local owned/`con`/`mut` method receiver와 argument overlap
- [x] field/index/temporary method receiver ownership
- [x] explicit struct/array/slice composite literal semantics
- [x] expected-type propagation into explicit literals through calls, returns,
  assignments, nested fields/elements and if-expression branches
- [x] expected-type propagation through `None`/`Some`/`Ok`/`Err`
- [x] user enum constructor payload semantics and expected-type propagation
- [x] flat Option/Result expression match expected types, coverage와 move join
- [x] flat Option/Result statement match return convergence와 move join
- [x] flat non-generic user enum pattern, payload binding과 exhaustive coverage
- [x] nested built-in/user enum pattern semantics와 recursive coverage
- [x] capture-free plain/mutable function literal과 structural callable signature
- [x] plain closure capture와 Copy/non-Copy move ownership
- [x] mutable/nested capture propagation과 closure ownership
- [x] plain/mutable/nested closure definition과 capture value typed IR
- [x] straight-line owned local/parameter drop과 deterministic return temporary
- [x] `if` branch-local tail/return cleanup
- [x] nested non-shadowing outer cleanup root branch join
- [x] branch shadow cleanup binding identity와 assignment reactivation
- [x] direct local cleanup overwrite RHS 선평가와 self-reassignment reactivation
- [x] non-self-consuming field/index cleanup overwrite와 aggregate base 보존
- [x] optimized generated-C full gate와 explicit `--fast` B2 inner loop
- [x] 영역별 `--focus`, bounded differential workers와 CI gate deduplication
- [x] selected project test를 위한 deterministic single-C runner와 per-case process isolation
- [x] `mut` cleanup parameter/capture external overwrite와 tail-drop 제외
- [x] self-consuming direct/indexed field `append` typed IR와 overwrite 제외
- [x] direct/field `append` move accounting과 non-Copy item consumption
- [x] read-only `len` typed IR와 return full-expression cleanup
- [x] statement-only `print` read argument와 owner tail cleanup
- [x] declaration-ordered struct literal typed IR와 field move cleanup
- [x] fixed-array/slice literal typed IR와 element move cleanup
- [x] built-in ADT constructor typed IR와 payload move cleanup
- [x] user enum constructor inline/owned storage와 payload cleanup
- [x] Copy built-in expression match arm/pattern typed IR
- [x] cleanup built-in expression match의 arm-local drop과 full-expression temporary
- [x] flat user-enum expression match의 inline/owned pattern과 payload cleanup
- [x] nested built-in/user-enum pattern과 recursive owned enum typed IR
- [x] statement-form match arm cleanup과 outer branch move compensation
- [x] test body semantic checking, `assert(bool)`와 synthetic test `main` typed IR
- [x] condition/conditionless `for`, `break`/`continue`와 loop block typed IR
- [x] three-clause `for` init/optional condition/post typed IR
- [x] Copy array/slice range binding, source와 body/cleanup typed IR
- [x] owned for-init exit 및 loop/range body-local control-flow cleanup
- [x] index-only non-Copy range와 indexed `con` element borrow typed IR
- [x] method declaration과 direct local owned/`con`/`mut` receiver typed IR
- [x] field/index/temporary/computed method receiver typed IR
- [x] demand-driven generic struct/function/receiver specialization과 typed IR
- [x] generic enum specialization과 source pattern origin 보존
- [x] symbolic generic body validation과 source diagnostic 복원
- [x] multi-source lexer/parser source ID 보존과 deterministic AST merge
- [x] project source path의 package identity와 package declaration 검증
- [x] same-project import validation, cycle rejection과 dependency-first package order
- [x] package declaration/method inventory, visibility와 generic parameter 보존
- [x] compiler-owned standard package registry, opaque/generic declaration inventory와 unknown rejection
- [x] cross-project source root, direct dependency edge와 undeclared transitive import rejection
- [x] package visibility 검증과 package-qualified declaration/type/body link rewrite
- [x] nested shadow cleanup binding identity를 C backend까지 보존
- [x] compiler-owned standard declaration augmentation과 intrinsic specialization semantics
- [x] full typed IR, deterministic drop insertion과 complete differential corpus
- [x] B2 canonical, publication과 supported-platform CI acceptance

Exact scope and decisions are owned by `docs/todo-self-hosting-semantics/`.
P176a freezes declaration/type normalization without changing public syntax or
the standard library. P176b1 adds primitive bodies and typed IR, and P176b2
adds direct/indirect calls and named function values. P176b3-P176e extend
places, nested scopes, ownership,
specialization and typed IR in independently differential-tested slices.

## P177: B3 Self-Hosting C Backend

상태: active; P177a, P177b1 complete (2026-07-22)

- [x] Mallang typed-IR read-only backend API
- [x] standalone Mallang `c` host mode
- [x] scalar function, call, binding, assignment과 return C emission
- [x] checked scalar arithmetic, comparison, logical operator와 statement-only print
- [x] Stage0/Stage1 generated C byte identity와 deterministic repeat
- [x] strict C11 native, allocation accounting과 ASan/UBSan scalar gate
- [x] match cleanup payload가 outer root를 claim하지 않는 Rust regression
- [x] artifact-reuse edit loop와 fresh integration/publication gate 분리
- [x] string runtime, UTF-8 literal/equality, owned return/local/drop과 full-expression cleanup
- [x] statement `if`/`else`, condition loop, `break`/`continue` C emission
- [x] scalar/owned-control 2-fixture byte identity, native, accounting과 sanitizer gate
- [x] expanded compiler source 725-function typed-IR parity
- [ ] string, array, slice, struct, ADT와 control-flow backend
- [ ] method, callable, closure, specialization과 project backend
- [ ] complete compiler-source C generation과 B3 canonical acceptance

Exact scope, gate layering and current evidence are owned by
`docs/todo-self-hosting-backend/`. P177a does not claim Stage2 generation or a
fixed point; those remain B4 work.

## Deferred 2.0 Naming Conventions

- [x] keep visibility controlled only by explicit `pub`
- [x] define role-based PascalCase, lowerCamelCase and lower_snake_case targets
- [x] keep `mlg fmt` syntax-preserving and non-renaming
- [ ] inventory current and ecosystem naming violations
- [ ] add compatible `mlg lint` warnings and machine-readable rule IDs
- [ ] add explicit resolver-backed `mlg fix --names`
- [ ] publish migration evidence before 2.0 compiler errors

The approved design and compatibility boundary are owned by
`docs/todo-naming-conventions/`. This debt does not change the frozen v1 source
contract or block the active B0-B5 self-hosting sequence.
