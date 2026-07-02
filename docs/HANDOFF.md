# Mallang Handoff

## 현재 상태

- 언어 이름: Mallang
- 소스 확장자: `.mlg`
- CLI: `mlg`
- 현재 구현: token model, hand-written lexer, AST, parser, semantic checker, ownership-lite move/borrow checks, borrowed non-copy parameter escape rejection, same-call nested-field-aware borrow conflict checks, string equality without moves, `bool` logical operators with native short-circuit smoke, `|>` pipeline call sugar, statement/expression `if`, condition-only `for` loops, conditionless `for` loops, `for init; condition; post` loops, array-only `for i, value := range values { ... }`, fixed-size array `values[i]` indexing for `Copy` elements with compile-time literal and native runtime bounds checks, fixed-size array `values[i] = expr` assignment for mutable `Copy` element arrays including `for` clause post targets, fixed-size array `len(values)`, `break`/`continue`, `else if` sugar, branch-aware return-completeness analysis, `type Name struct` declarations, named struct literals, nested field access, nested mutable field assignment, nested field-level borrow arguments, read/mut struct receiver methods, generic type refs, fixed-size array type refs and fixed-size array literals type-checked, fixed-size arrays as move-only values, fixed-size array typed IR/C struct-wrapper layout, `Option`/`Result` constructor type checking, exhaustive expression/statement `match` checking, statement-form `match` block arms, non-local `match` scrutinee temp codegen, `if` expression branch prelude temp codegen, `match` expression arm prelude temp codegen, tagged ADT typed IR/backend layout, printable `Option`/`Result` native output, printable struct native output, typed IR, first native subset C backend, hidden-reference C ABI for `in`/`mut` parameters, caller-visible `mut` parameter mutation, `mlg check`, `mlg ir`, `mlg build`, `Option`/`Result` surface spec
- 아직 없음: slice surface syntax, borrowed/non-copy indexing, non-copy array element assignment, statement-spanning borrow lifetimes, for-clause header lowering for complex expressions that need temporary preludes, full C backend, method values/interfaces/dynamic dispatch

## 빠른 시작

```sh
scripts/check.sh
cargo run --bin mlg -- check examples/first.mlg
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
cargo run --bin mlg -- check examples/arrays.mlg
cargo run --bin mlg -- ir examples/arrays.mlg
cargo run --bin mlg -- build examples/arrays.mlg -o target/mallang/arrays
target/mallang/arrays
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

- `SPEC.md`: 언어 v0 설계 초안
- `ROADMAP.md`: compiler milestone
- `docs/ROADMAP.md`: agent가 다음 작업을 고르는 운영용 roadmap
- `docs/REPO_MANIFEST.yaml`: 검증 명령과 entrypoint 선언
- `docs/ESCALATION_POLICY.md`: 사용자 호출 조건

## 다음 구현 후보

1. slice `[]T`, append/growth, mutable range value의 ownership surface 결정
2. borrowed/non-copy indexing과 non-copy array element assignment의 ownership boundary 결정
3. statement-spanning borrow lifetimes가 필요한 syntax가 생기는지 점검
4. for-clause header prelude lowering을 statement-lowering으로 풀지 결정
5. full C backend 범위를 native subset별로 쪼개기
6. method values/interfaces/dynamic dispatch를 v0 이후로 미루는 결정 확정
