# Naming Conventions Design Debt

Status: active; P181 compatible naming lint implementation

## Problem

Mallang v1 treats identifier case as spelling, except for the existing
lowercase project-name grammar. Visibility is explicit through `pub`, but the
language does not yet define or enforce role-based casing for source symbols.
Leaving casing entirely conventional would make formatter output and public
APIs inconsistent across projects.

Go's uppercase-export rule is compact and easy to scan, but it couples two
independent decisions. Renaming a symbol changes visibility, and changing
visibility requires a reference-wide rename. It also prevents private and
public types from sharing one type-oriented naming rule. Mallang keeps these
concerns orthogonal:

- `pub` is the only source-level visibility control.
- Identifier casing describes the declaration or binding role.
- Changing visibility must not require renaming a symbol.
- Changing case must never silently change visibility.

## Target Rules

| Symbol role | Target spelling | Examples |
| --- | --- | --- |
| struct and enum types | `PascalCase` | `User`, `HttpClient` |
| enum variants | `PascalCase` | `Some`, `RequestFailed` |
| type parameters | `PascalCase` | `T`, `Item`, `ErrorType` |
| functions and methods | `lowerCamelCase` | `newUser`, `parseUtf8` |
| locals and parameters | `lowerCamelCase` | `userName`, `retryCount` |
| receiver and field names | `lowerCamelCase` | `self`, `createdAt` |
| test declaration names | `PascalCase` | `ParsesRange`, `RejectsMove` |
| package identifiers | `lower_snake_case` | `main`, `parser_control` |
| project names and path segments | `lower_snake_case` after migration | `bootstrap_compiler` |

Acronyms are treated as words: `HttpClient`, `parseUtf8`, and `apiUrl` are
preferred over mixed initialism rules such as `HTTPClient` or `parseUTF8`.
`main` remains the required entrypoint spelling, and `_` remains the blank
identifier rather than an ordinary binding.

Both private and public declarations follow the same role rule:

```mlg
type InternalUser struct {}
pub type User struct {}

func normalizeName() {}
pub func newUser() User {}
```

## Compatibility

These rules are not v1 acceptance rules. Existing 1.x public functions such as
`Lex`, `Parse`, `Check`, `Normalize`, and `CopyText`, existing PascalCase test
names, and lowercase project names containing hyphens remain source-compatible.
Silently rejecting or renaming them in 1.x would violate
`docs/COMPATIBILITY.md`.

The migration sequence is:

1. Inventory repository and ecosystem spellings, including generated and
   cross-package references.
2. Add a compatible `mlg lint` naming-warning surface with stable symbol-role
   diagnostics and explicit legacy suppression.
3. Add opt-in, resolver-backed `mlg fix --names` only when every rewritten
   reference and public API change can be reported before applying it.
4. Publish a 2.0 migration and deprecation window before making violations
   compiler errors.

`mlg fmt` must never rename identifiers. Formatting is syntax-preserving;
reference-aware renaming belongs to an explicit fix or IDE code action.

## Acceptance Boundaries

- Visibility tests prove casing has no effect on package access and only `pub`
  changes exposure.
- Lint tests classify every symbol role, acronym policy, entrypoint exception,
  blank identifier and project/package boundary deterministically.
- Fix tests cover local shadowing, methods, generic parameters, enum patterns,
  imports, cross-package references and public API change previews.
- The 2.0 hard-error gate includes an exact migration corpus and never relies
  on formatter-side rewriting.

## Deferred Work

- [x] Separate visibility from role-based casing.
- [x] Record the target casing matrix and 1.x compatibility boundary.
- [x] Reserve formatter behavior as non-renaming.
- [ ] Inventory current violations and publish lint rule IDs.
- [ ] Implement `mlg lint` warnings and machine-readable diagnostics.
- [ ] Implement explicit reference-aware naming fixes.
- [ ] Plan and verify the 2.0 hard-error migration.
