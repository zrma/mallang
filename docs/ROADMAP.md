# Agent Roadmap

## P0: Bootstrap

- [x] Mallang naming 정리
- [x] Rust crate 생성
- [x] lexer/token model 추가
- [x] repo 관리 문서와 검증 스크립트 추가
- [ ] GitHub repo publish

## P1: Parser Frontend

- [ ] AST module 추가
- [ ] function declaration parser 추가
- [ ] block/statement parser 추가
- [ ] Pratt expression parser 추가
- [ ] `examples/hello.mlg`를 AST로 파싱하는 test 추가

## P2: Static Semantics

- [ ] name resolver 추가
- [ ] primitive type checker 추가
- [ ] function signature checker 추가
- [ ] immutable binding reassignment reject

## P3: Ownership Lite

- [ ] Copy/move type classification 추가
- [ ] use-after-move reject
- [ ] `in` read borrow call rule 추가
- [ ] `mut` exclusive borrow call rule 추가

## P4: Native Backend

- [ ] typed IR 추가
- [ ] C codegen 추가
- [ ] `mlg build` subcommand 추가
- [ ] `clang` 기반 native binary smoke 추가
