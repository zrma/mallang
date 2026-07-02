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
- Equality operators `==` and `!=` support `int`, `bool`, and `string`. String
  equality compares contents and does not move the compared values.
- Logical operators `&&` and `||` support `bool` operands and use
  short-circuit evaluation in the native backend.

Reserved words:

```text
func return if else for break continue match case mut in true false struct type nil
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
string struct
```

Arrays and slices are reserved for a later v0 slice and are expected to be
move-only unless explicitly designed otherwise.

`unit` is the implicit return type of functions that do not return a value.

## Structs

Struct declarations use a Go-like `type Name struct` form.

```go
type User struct {
    name string
    age int
}
```

Struct literals use named fields.

```go
user := User{name: "kim", age: 30}
user.age = 31
print(user.name)
print(user.age)
```

Nested field paths are allowed when each step is a struct field.

```go
type Profile struct {
    displayName string
}

type Account struct {
    profile Profile
}

account := Account{profile: Profile{displayName: "kim"}}
account.profile.displayName = "lee"
print(account.profile.displayName)
```

Rules:

- Struct values are move-only in v0.
- Literal fields must name every declared field exactly once.
- Field access reads a field by name.
- Field assignment updates a field path rooted in a mutable local struct binding.
- Copy fields such as `int` and `bool` can be read as values.
- Non-copy fields such as `string` can be borrowed for calls like `print`, but
  moving a non-copy field out of a named struct is rejected until destructuring
  or partial-move semantics is designed.
- Field paths rooted in local bindings can be used as borrow arguments, such as
  `show(in user.name)` or `rename(mut user.profile.name)`.
- Mutable field borrow arguments require the root binding to be `mut`.
- Borrow conflict checks are field-aware within a single call: overlapping
  whole-struct/field borrows and same-field exclusive borrows are rejected,
  while disjoint mutable field borrows such as `mut pair.left`,
  `mut pair.right`, `mut user.name.first`, and `mut user.name.last` are allowed.
- `print` displays struct values as `Type{field: value, ...}` when every field
  type is printable in the native backend.
- v0 does not include recursive by-value structs or struct pattern matching.

Methods use Go-like receiver declarations with Mallang's existing parameter
mode syntax.

```go
func (self in User) age() int {
    return self.age
}

func (self mut Counter) inc() {
    self.value = self.value + 1
}

print(user.age())
```

Rules:

- The receiver must be a struct type in v0.
- Receiver modes are the same as parameter modes: owned, `in`, and `mut`.
- A method call implicitly passes the receiver according to the method
  declaration.
- Returning or storing borrowed values is still unsupported, so methods with
  `in` receivers cannot return non-copy fields such as `string` by value.
- v0 does not include method values, interfaces, dynamic dispatch, or receiver
  overloading outside concrete struct receivers.

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
readField(in user.name)
renameField(mut user.profile.name)
consume(user)
```

Rules:

- Passing a non-copy value as `T` moves ownership into the callee.
- Passing `in T` creates a read-only borrow for the duration of the call.
- Passing `mut T` creates an exclusive mutable borrow for the duration of the call.
- Native code passes `in T` and `mut T` as hidden references. Inside the callee,
  reads use normal value syntax, and assignment through a `mut T` parameter
  updates the caller's local variable or field path.
- Borrow arguments may be local variables or field paths rooted in local variables.
- Non-copy borrowed parameters cannot be moved into owned locals, owned
  arguments, or return values. Copy borrowed parameters such as `int` and `bool`
  may still be copied out as values.
- Borrowed values cannot be stored in variables in v0.
- Borrowed values cannot be returned in v0.
- Non-`unit` functions must return on every path the v0 checker can prove.

## Expressions

`if` is both a statement and an expression.

Statement form:

```go
if enabled {
    print("on")
} else {
    print("off")
}
```

Statement rules:

- The condition must have type `bool`.
- `else` is optional for statement-form `if`.
- Bindings introduced inside a branch do not leak outside the branch.
- Moving an outer value inside either branch makes the value unavailable after
  the statement in v0.
- An `if` statement is return-complete only when both `then` and `else` blocks
  are return-complete.
- `else if` is sugar for a nested statement-form `if` inside the `else` branch.

`for` is a statement. v0 supports a condition-only form, matching Go's
`while`-like loop shape, and a small Go-like three-clause form.

```go
for enabled {
    tick()
}

for mut i := 0; i < 3; i = i + 1 {
    print(i)
}
```

Rules:

- The condition must have type `bool`.
- In the three-clause form, init is `name := expr` or `mut name := expr`.
- In the three-clause form, post is a single variable or field assignment.
- Bindings introduced by the init clause are scoped to the loop header and body.
- Bindings introduced inside the body do not leak outside the body.
- Moving an outer value inside the body makes the value unavailable after the
  loop in v0.
- `break` exits the nearest enclosing loop.
- `continue` skips to the next iteration of the nearest enclosing loop.
- `break` and `continue` are only valid inside loops.
- A `for` statement is not considered return-complete in v0, even when its
  condition is statically `true`.
- v0 does not yet include `range`, initless clause loops, empty conditions, or
  post declarations.

Expression form:

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
- Expression branches must produce a value.
- `else if` is sugar for a nested `if` expression in the else branch.

Pipeline expressions use a functional value-first style.

```go
7 |> double() |> add(1) |> print()
```

Rules:

- `value |> f(args...)` is call sugar for `f(value, args...)`.
- The pipeline target must currently be a direct call.
- The piped value is passed as an owned first argument in v0. Borrow-mode
  pipeline syntax is deferred.

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

`Option` and `Result` are built-in algebraic data types in the v0 language
model. The implementation may specialize them internally, but user code treats
them as ordinary value types.

Type syntax:

```text
Option[T]
Result[T, E]
```

Constructors:

```go
Some(value)
None
Ok(value)
Err(error)
```

Rules:

- `Some(value)` has type `Option[T]` when `value` has type `T`.
- `Ok(value)` has type `Result[T, E]` when `value` has type `T` and `E` is
  known from context.
- `Err(error)` has type `Result[T, E]` when `error` has type `E` and `T` is
  known from context.
- `None` requires an expected type from return type, binding annotation, or
  surrounding expression context.
- `Option[T]` is `Copy` only when `T` is `Copy`; otherwise it is move-only.
- `Result[T, E]` is `Copy` only when both `T` and `E` are `Copy`; otherwise it
  is move-only.
- Matching a move-only payload moves it into the matched binding unless a future
  borrowed pattern form is introduced.
- `match` scrutinees are evaluated once, including expression scrutinees such as
  direct function calls.
- `print` displays ADT values as `Some(value)`, `None`, `Ok(value)`, and
  `Err(value)` when the payload type itself is printable in the native backend.
- v0 does not include `unwrap`, `?`, nested patterns, or user-defined enum
  declarations.

Implementation staging:

1. Parse generic type references (`Option[T]`, `Result[T, E]`).
2. Add type-directed constructors for `Some`, `None`, `Ok`, and `Err`.
3. Add exhaustive `match` checking for `Option` and `Result`.
4. Lower ADTs into typed IR as tagged values.
5. Specialize C backend layouts per concrete instantiation.
6. Print ADT values with printable payloads in the native backend.

## Match

`match` destructures algebraic data types.

```go
label := match user {
    case Some(u) { u.name }
    case None { "not found" }
}

match user {
    case Some(u) {
        print(u.name)
    }
    case None {
        print("not found")
    }
}
```

Rules:

- v0 `match` must be exhaustive for `Option` and `Result`.
- Pattern guards are deferred.
- Nested patterns are deferred.
- Matching `Option[T]` requires exactly `Some(name)` and `None` arms.
- Matching `Result[T, E]` requires exactly `Ok(name)` and `Err(name)` arms.
- All arms of a `match` expression must produce the same non-`unit` type.
- Statement-form `match` arms are blocks and may contain multiple statements.
- A statement-form `match` is return-complete when every arm block is
  return-complete.
- Matching a move-only scrutinee consumes it.

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
