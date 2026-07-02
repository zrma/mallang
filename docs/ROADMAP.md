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
- [x] `mlg check` subcommand 추가

## P3: Ownership Lite

- [x] Copy/move type classification 추가
- [x] use-after-move reject
- [x] `in` read borrow call rule 추가
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
- [x] `in`/`mut` parameter hidden-reference C ABI 추가
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
- [ ] slice `[]T`, indexing, `len`, append/growth, mutable range는 후속 slice로 분리
