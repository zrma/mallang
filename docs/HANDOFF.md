# Mallang Handoff

## 현재 상태

- 언어 이름: Mallang
- 소스 확장자: `.mlg`
- CLI: `mlg`
- 현재 구현: token model, hand-written lexer, AST, parser, semantic checker, entrypoint `func main()` signature checks, ownership-lite move/borrow checks, borrowed non-copy parameter escape rejection, same-call nested-field-aware borrow conflict checks, built-in value name collision checks, top-level type/function declaration name conflict checks, nested-block and arm-local shadowing with same-block redeclaration rejection, string equality without moves, guarded integer division/remainder, checked integer arithmetic, semantic printability checks, statement-only `print` semantic checks, `bool` operators with native short-circuit smoke, `|>` pipeline call sugar, statement/expression `if`, condition-only `for` loops, conditionless `for` loops, `for init; condition; post` loops, array/slice `for i, value := range values { ... }`, blank identifiers and one-variable forms in range loops, fixed-size array `values[i]` indexing for `Copy` elements with compile-time literal and native runtime bounds checks, borrowed indexing expressions for read-only non-Copy element inspection, fixed-size array `values[i] = expr` assignment for mutable `Copy` and non-copy element arrays including `for` clause post targets, fixed-size array `len(values)`, source-level owned slice type syntax `[]T`, slice literals `[]T{...}`, `len(slice)`, Copy-only `slice[i]` value access, consuming built-in `append(slice, item) -> []T` with native realloc growth, same-field append reassignment for direct and stable indexed owned slice field paths, field-take append sources and general owned value position takes for owned slice fields, slice range with Copy value iteration and index-only non-Copy iteration, local-rooted slice field/index len/index/range/borrow reads, slice element borrow arguments for local-rooted owned slices, slice element assignment for local-rooted mutable owned slices, indexed field assignment for array/slice elements, struct cleanup for owned slice fields, internal owned slice `Type::Slice` / C `{data,len,cap}` shell and cleanup classification, internal cleanup type `mlg_drop_*` helper emission shell, explicit internal `IrStmtKind::Drop` backend lowering, straight-line cleanup param/local drop insertion before tail/return/reassignment, branch-local cleanup drop insertion for `if`/`match` statement bodies, outer cleanup root branch move normalization for `if`/`match` statements, expression-form `if`/`match` branch cleanup normalization, loop body-local cleanup drop insertion for `for`/`range` tail and `break`/`continue` paths, `for` init cleanup trailer lowering, `break`/`continue`, `else if` sugar, branch-aware return-completeness analysis, `type Name struct` declarations, named struct literals, recursive struct value-type rejection, nested field access, nested mutable field assignment, nested field-level borrow arguments, fixed-size array element borrow arguments, fixed-size array element method receivers, con/mut struct receiver methods, generic type refs, fixed-size array type refs and fixed-size array literals type-checked, fixed-size arrays as move-only values, fixed-size array typed IR/C struct-wrapper layout, `for`/`range` body C block lowering for shadowed locals, `Option`/`Result` constructor type checking, exhaustive expression/statement `match` checking, statement-form `match` block arms, non-local `match` scrutinee temp codegen, `if` expression branch prelude temp codegen, `match` expression arm prelude temp codegen, `for` clause condition/post prelude codegen with post-preserving `continue`, tagged ADT typed IR/backend layout, printable `Option`/`Result` native output, printable struct native output, typed IR, backend public API boundary with C implementation, name helper, type emitter, statement emitter, expression emitter, and shared utility modules, first native subset C backend, hidden-reference C ABI for `con`/`mut` parameters, caller-visible `mut` parameter mutation, `mlg check`, `mlg ir`, `mlg build`, `mlg run`, `Option`/`Result` surface spec
- 아직 없음: first-class borrowed references, statement-spanning borrow lifetimes, general partial moves from fields beyond slice field take, full C backend, method values/interfaces/dynamic dispatch

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

- `SPEC.md`: 언어 v0 설계 초안
- `ROADMAP.md`: compiler milestone
- `docs/ROADMAP.md`: agent가 다음 작업을 고르는 운영용 roadmap
- `docs/REPO_MANIFEST.yaml`: 검증 명령과 entrypoint 선언
- `docs/ESCALATION_POLICY.md`: 사용자 호출 조건

## 다음 구현 후보

1. general field partial move 설계 beyond slice field take
2. statement-spanning borrow lifetimes가 필요한 syntax가 생기는지 점검
3. C backend orchestration/test layout를 더 얇게 유지할지 결정
