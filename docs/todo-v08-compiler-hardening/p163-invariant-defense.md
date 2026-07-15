# P163: Compiler and IR Invariant Defense

상태: complete (2026-07-16); P164 next

## Audit result

Production `panic!`, `unreachable!`, `unwrap`, `expect`와 direct indexing을 frontend, project,
package/link, semantic, specialization, IR, backend와 CLI 경계에서 다시 분류했다. Blanket
`catch_unwind`는 추가하지 않았다.

| Boundary | Classification | Defense |
| --- | --- | --- |
| direct `Parser::new` token input | user/API reachable | missing EOF is normalized to a synthetic sentinel; empty and sentinel-less streams are regression-tested |
| parsed match pattern extraction | user-adjacent | non-empty segment extraction now returns `ParseError` instead of using `expect` |
| method receiver diagnostic span | user-adjacent | the already matched receiver node supplies the span without a second `unwrap` |
| empty expression/block match arms | user reachable | semantic analysis returns `match requires at least one arm`; IR lowering independently rejects a malformed checked program |
| frontend/package/semantic malformed source | user reachable | compiler regression fixes the owning `CompilerStage` and returned diagnostic |
| typed IR declaration graph | malformed internal IR | backend preflight rejects duplicate types, fields, variants, functions, parameters and closure captures, plus an invalid `main` signature |
| expression/statement backend invariants | malformed internal IR | existing local validators continue to reject invalid print/range/match/constructor/borrow/drop nodes through `CompileError` |
| source/package/project graph indices | locally proven | indices and stack entries are created by the same traversal before lookup; malformed paths/imports return their owning error type first |
| parser cursor indexing | locally proven | constructor-owned EOF normalization and non-advancing EOF behavior keep `peek`/`advance` in bounds |
| string/JSON formatting | locally proven | writes target `String`, and `Diagnostic` contains only serializable owned scalar fields |

## Backend preflight contract

`generate_c_from_ir` runs declaration validation before writing the C preamble. It preserves the
existing fragment contract: an IR program may omit `main`, but if `main` exists it must have no
parameters and return `unit`. Duplicate declarations that would otherwise produce colliding C
symbols or fields fail with an `IR invariant violation` diagnostic.

The validator does not duplicate every expression emitter check. Shape-dependent nodes stay close
to their emitter, where the full expected type and call environment already exist. Negative tests
cover both preflight failures and these local checks.

## Acceptance evidence

- direct parser input without an EOF token and an empty token stream complete without a panic
- malformed source cases stop at frontend, package or semantic diagnostics
- empty `match` arms produce a semantic diagnostic
- duplicate IR function/struct field and invalid `main` signature fail before C emission
- the existing invalid print, range, ADT, borrow and drop IR matrix remains green
- `cargo test --lib`: 575 passed

P164 owns deterministic source/token mutation, type/ownership negative corpus expansion and every
newly discovered minimized crash fixture.
