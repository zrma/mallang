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
- [x] suffix mode `name con T` / `name mut T` reject 추가
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

- [x] direct local slice source의 `con values[i]` / `mut values[i]` 허용
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
