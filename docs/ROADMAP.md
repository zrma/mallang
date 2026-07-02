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
- [x] `examples/first.mlg`를 AST로 파싱하는 test 추가

## P2: Static Semantics

- [x] first native subset용 name resolver 추가
- [x] first native subset용 primitive type checker 추가
- [x] first native subset용 function signature checker 추가
- [x] immutable binding reassignment reject
- [x] `if` expression type checking 추가
- [x] statement-form `if` type checking 추가
- [x] statement-form `if` return-completeness analysis 추가
- [x] `mlg check` subcommand 추가

## P3: Ownership Lite

- [x] Copy/move type classification 추가
- [x] use-after-move reject
- [x] `in` read borrow call rule 추가
- [x] `mut` exclusive borrow call rule 추가
- [x] same-call overlapping borrow tracking 추가
- [ ] borrow return/storage 금지 규칙을 reference 타입 도입 시 검증

## P4: Native Backend

- [x] typed IR 추가
- [x] `if` expression typed IR/codegen 추가
- [x] first native subset용 C codegen 추가
- [x] `mlg build` subcommand 추가
- [x] `clang` 기반 native binary smoke 추가
- [x] statement-form `if` C codegen/native smoke 추가

## P5: Built-in ADTs

- [x] `Option[T]` / `Result[T, E]` surface 설계
- [x] generic type reference parser 추가
- [x] `Some` / `None` / `Ok` / `Err` constructor type checking 추가
- [x] `Option` / `Result` exhaustive `match` 추가
- [x] tagged typed IR와 C backend layout 추가
- [x] non-local `match` scrutinee temp codegen 추가

## P6: Structs

- [x] `type Name struct { ... }` parser/semantic 추가
- [x] named struct literal과 field access 추가
- [x] struct typed IR와 C backend typedef/literal/access 추가
- [x] struct receiver methods 설계/구현
- [x] direct mutable field assignment 추가
- [x] field-level borrow arguments 추가
- [x] nested field assignment와 nested field borrow argument 추가
