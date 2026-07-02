# Mallang Handoff

## 현재 상태

- 언어 이름: Mallang
- 소스 확장자: `.mlg`
- CLI: `mlg`
- 현재 구현: token model, hand-written lexer, AST, parser, semantic checker, ownership-lite move/borrow checks, same-call field-aware borrow conflict checks, statement/expression `if`, `type Name struct` declarations, named struct literals, field access, direct mutable field assignment, direct field-level borrow arguments, struct receiver methods, generic type refs, `Option`/`Result` constructor type checking, exhaustive `match` expression checking, non-local `match` scrutinee temp codegen, tagged ADT typed IR/backend layout, typed IR, first native subset C backend, `mlg check`, `mlg ir`, `mlg build`, `Option`/`Result` surface spec
- 아직 없음: statement-spanning borrow lifetimes, full C backend, `else if` sugar, return-completeness analysis across branches, statement-block `match` arms, nested field assignment, nested field borrow arguments, by-reference native ABI for caller-visible `mut` parameter mutation, method values/interfaces/dynamic dispatch

## 빠른 시작

```sh
scripts/check.sh
cargo run --bin mlg -- check examples/first.mlg
cargo run --bin mlg -- ir examples/adt.mlg
cargo run --bin mlg -- build examples/first.mlg -o target/mallang/first
target/mallang/first
cargo run --bin mlg -- build examples/if-statement.mlg -o target/mallang/if-statement
target/mallang/if-statement
cargo run --bin mlg -- build examples/adt.mlg -o target/mallang/adt
target/mallang/adt
cargo run --bin mlg -- build examples/match-temp.mlg -o target/mallang/match-temp
target/mallang/match-temp
cargo run --bin mlg -- build examples/structs.mlg -o target/mallang/structs
target/mallang/structs
cargo run --bin mlg -- build examples/methods.mlg -o target/mallang/methods
target/mallang/methods
cargo run --bin mlg -- build examples/field-assignment.mlg -o target/mallang/field-assignment
target/mallang/field-assignment
cargo run --bin mlg -- build examples/field-borrow.mlg -o target/mallang/field-borrow
target/mallang/field-borrow
```

## 주요 문서

- `SPEC.md`: 언어 v0 설계 초안
- `ROADMAP.md`: compiler milestone
- `docs/ROADMAP.md`: agent가 다음 작업을 고르는 운영용 roadmap
- `docs/REPO_MANIFEST.yaml`: 검증 명령과 entrypoint 선언
- `docs/ESCALATION_POLICY.md`: 사용자 호출 조건

## 다음 구현 후보

1. nested field assignment와 nested field borrow argument 규칙 설계
2. statement-spanning borrow lifetimes가 필요한 syntax가 생기는지 점검
3. `else if` sugar와 return-completeness analysis 적용 시점 결정
4. statement-block `match` arms 필요 시점 결정
5. method values/interfaces/dynamic dispatch를 v0 이후로 미루는 결정 확정
