# Mallang Specification Draft

This is the v0 design target for the PoC. Syntax and names are provisional.

## Naming

The language name is Mallang.

```text
source extension: .mlg
user-facing command: mlg
build command: mlg build
run command: mlg run
check command: mlg check
```

`mallang` is reserved for documentation, package names, and searchability. The
day-to-day command is intentionally short.

## Design Goals

- Keep Go's readable declaration and block syntax.
- Remove raw pointers, `nil`, and ambient memory unsafety.
- Use Rust-inspired ownership and borrowing without exposing lifetimes in v0.
- Encourage functional, value-oriented code.
- Compile to native binaries.
- Start with a C backend before committing to LLVM or Cranelift.

## Non-goals for v0

- No goroutines.
- No interfaces.
- No generics beyond built-in `Option[T]` and `Result[T, E]` planning.
- No closures.
- No user-visible lifetimes.
- No garbage collector.
- No raw pointer or address-of syntax.

## Source Shape

```go
func main() {
    name := "kim"
    msg := greet(in name)
    print(msg)
}

func greet(name in string) string {
    return "hello " + name
}
```

## Lexical Rules

- Identifiers use ASCII letters, digits, and `_`.
- Identifiers cannot start with a digit.
- Line comments start with `//`.
- String literals use double quotes.
- Integer literals are base-10 in v0.

Reserved words:

```text
func return if else match case mut in true false struct type nil
```

`nil` is reserved so the compiler can produce a clear error instead of treating
it as an ordinary identifier.

## Bindings

Bindings are immutable by default.

```go
x := 10
mut y := 20
y = y + x
```

Rules:

- Reassignment requires a `mut` local binding.
- Shadowing is allowed only in a nested block.
- v0 does not support uninitialized locals.

## Types

Primitive v0 types:

```text
int
bool
string
unit
```

Copy types:

```text
int bool
```

Move types:

```text
string array struct
```

`unit` is the implicit return type of functions that do not return a value.

## Functions

Function declarations use Go-like syntax.

```go
func add(a int, b int) int {
    return a + b
}
```

Parameter modes:

```text
name T      // owned value
name in T   // read borrow
name mut T  // mutable borrow
```

Call sites must make borrow mode explicit.

```go
readName(in user)
rename(mut user, "lee")
consume(user)
```

Rules:

- Passing a non-copy value as `T` moves ownership into the callee.
- Passing `in T` creates a read-only borrow for the duration of the call.
- Passing `mut T` creates an exclusive mutable borrow for the duration of the call.
- Borrowed values cannot be stored in variables in v0.
- Borrowed values cannot be returned in v0.

## Expressions

`if` is both a statement and an expression.

```go
label := if score >= 60 {
    "pass"
} else {
    "fail"
}
```

Rules:

- `if` used as an expression requires `else`.
- Both branches must have the same type.

## Option and Result

The language has no `nil`. Optional values use `Option[T]`.

```go
func findUser(id int) Option[User] {
    if id == 1 {
        return Some(User{name: "kim"})
    }

    return None
}
```

Recoverable errors use `Result[T, E]`.

```go
func readConfig(path string) Result[Config, Error] {
    // ...
}
```

`Option` and `Result` are planned as built-in algebraic data types.

## Match

`match` destructures algebraic data types.

```go
match user {
case Some(u):
    print(u.name)
case None:
    print("not found")
}
```

Rules:

- v0 `match` must be exhaustive for `Option` and `Result`.
- Pattern guards are deferred.
- Nested patterns are deferred.

## Ownership Rules

Initial rule set:

- Every value has one owner unless it is `Copy`.
- Assigning or passing a move value transfers ownership.
- Moved variables cannot be used.
- Any number of read borrows may exist at once.
- A mutable borrow is exclusive.
- Mutation requires a mutable binding or mutable borrow.
- References are not first-class values in v0.

This keeps the first checker tractable while preserving the main safety
properties: no dangling references, no use-after-free, and no data races within
single-threaded v0 execution.

## Backend Strategy

Initial native path:

```text
source -> tokens -> AST -> typed AST -> ownership checked IR -> C -> clang -> native binary
```

The C backend is an implementation tool, not the language's semantic model.
