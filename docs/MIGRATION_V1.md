# Migrating 0.x Source and Workflows to Mallang v1

This guide covers repository-observed source and workflow changes from the
bootstrap period through v0.8. It does not invent aliases for experimental forms
that were removed before v0.1.0.

## Compatibility summary

No published v0.1.0 language form was intentionally removed through v0.8.0.
Most milestones added projects, closures, generic ADTs, runtime ownership,
standard packages, and tooling. The source-breaking reservations introduced
after v0.1 are the compiler-owned project/symbol namespaces described below.

Early bootstrap checkouts briefly evaluated borrow spellings that never became a
published release contract. They still have an explicit mechanical migration so
unreleased source does not need a compatibility alias.

## Borrow modes

Use prefix-only `con` and `mut` in declarations and calls:

```mlg
func read(con name string) {
    print(name)
}

func rename(mut name string) {
    name = "lee"
}

func main() {
    mut name := "kim"
    read(con name)
    rename(mut name)
}
```

Rewrite bootstrap experiments as follows:

| Old experiment | v1 form |
| --- | --- |
| `func read(name in string)` | `func read(con name string)` |
| `func rename(name mut string)` | `func rename(mut name string)` |
| `read(in name)` | `read(con name)` |
| no marker | owned parameter or argument; a non-Copy value moves |

`con expr` and `mut expr` are direct call arguments, not values. Do not bind,
return, store, or capture them.

## Non-Copy range traversal

Borrowed or mutable range value bindings are not v1 syntax:

```mlg
// invalid
for i, con value := range values {}
for i, mut value := range values {}
```

Use index-only range and call-scoped indexed access:

```mlg
for i := range values {
    inspect(con values[i])
    update(mut values[i])
}
```

## Standalone source to project

A direct `.mlg` file remains valid and needs no manifest. To migrate to a
multi-package executable:

1. Create `mallang.toml` with `[project]` and a lowercase project path name.
2. Move the entry source to `src/main.mlg` and add `package main`.
3. Put each imported package in `src/<path>/` with a matching package declaration.
4. Use `pub` for every cross-package type or function and
   `import "project/path"` for the consumer.

Local dependencies use exact-name relative entries:

```toml
[dependencies]
model = { path = "../model" }
```

The key must equal the dependency's project name. Aliases, remote dependencies,
and importing an undeclared transitive dependency are invalid.

## Generic and algebraic data types

Built-in `Option` and `Result` keep their unqualified constructor and pattern
spelling:

```mlg
Some(value)
None
Ok(value)
Err(error)
```

Do not rewrite them as `Option.Some` or `Result.Ok`. User-defined enum variants
are qualified, including concrete generic arguments where required:

```mlg
Maybe[int].Some(1)
case Maybe.Some(value) { value }
```

Nested patterns and productive recursive user enums are supported. Raw pointers,
explicit `Box`/heap wrappers, and reference syntax are not migration targets;
recursive enum indirection is compiler-owned.

## Ownership changes to account for

- Function values, closures, arrays, slices, structs, user enums, maps, and
  strings are move-only unless the contract explicitly classifies a value Copy.
- General non-slice field moves remain invalid. Taking an owned slice field is
  the sole exception and resets the source field to an empty slice.
- Replace an owned value through a mutable local, field, `mut` parameter, or
  `mut` receiver. There is no implicit clone or shared heap owner.
- Fatal runtime guards do not unwind. Recoverable standard-library failures use
  `Result` and exhaustive `match`; v1 has no `?` or exceptions.

## Compiler-owned names introduced in v0.6

Projects created before v0.6 must be renamed if their project name is `std`.
User identifiers beginning with `__mlg_` must also be renamed. The `std/...`
package namespace and `__mlg_` symbol prefix are compiler-owned in v1.

## Tests, diagnostics, and installation

- Project tests live under `tests/`, mirror `src/` package directories, and use
  `test Name() { ... }` with statement-only `assert(bool)`.
- Automation that parsed human diagnostics should use
  `mlg --diagnostic-format json check <input>` and schema
  `mallang.diagnostic.v1`. Exact human wording is not stable.
- `mlg fmt --check <input>` is the no-write formatting gate.
- v0.7 and later binary releases install with an explicit version. Re-running
  the installer with another version is both the upgrade and rollback workflow.

## Migration verification

Run the checked-in canonical and rejection fixtures:

```sh
cargo build --bin mlg
scripts/check-v1-migration.sh target/debug/mlg
python3 scripts/check-v1-conformance.py
```

Then verify the migrated project end to end:

```sh
mlg fmt --check <project>
mlg check <project>
mlg test <project>
mlg build <project> -o <binary>
<binary>
```
