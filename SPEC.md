# Mallang v0.6 Specification

This is the Mallang language specification through the 0.6.0 source release.

Later milestones are non-normative plans tracked in `docs/V1_ROADMAP.md`.

## Published v0.2 Project Model

The approved v0.2 project surface extends the existing standalone `.mlg` mode
without changing it:

- A project has a `mallang.toml` manifest with a lowercase project path name.
- Project sources live under `src/`; `src/main.mlg` is the executable entry
  source.
- Every project source declares `package <name>`. Files in the same directory
  form one package and must use the same package name.
- `import "project/path"` imports a package. The final path segment is its
  qualifier. Imported functions and types use that qualifier, such as
  `greet.Print()` and `greet.Message`.
- Top-level declarations are package-private by default and use explicit `pub`
  when another package may access them. A public declaration cannot expose a
  package-private type in its fields, parameters, receiver, or return type.
- A receiver method may only be declared on a struct from the same package.
- Directory and manifest inputs select project mode. A direct `.mlg` input
  continues to select manifest-free standalone mode, including inside a project.
- Project source discovery and package graph processing are deterministic. Any
  import cycle is rejected in v0.2.

The v0.2 surface intentionally excludes import aliases, dot or wildcard imports,
remote dependencies, a package registry, lockfiles, and package initialization
hooks. The compiler and native acceptance path implement these normative rules.

## Implemented v0.6 Standard Library

The approved v0.6 standard-library contract implements its compiler foundation,
UTF-8 text, process, standard-stream, file, and owned-map slices. Exact public
signatures are listed in `docs/STANDARD_LIBRARY.md`.

- `import "std/..."` works in project and manifest-free standalone source for
  the six approved standard packages.
- Project name `std` and identifiers beginning with `__mlg_` are reserved for
  compiler-owned packages and symbols.
- Standard declarations use exact public signatures and ordinary argument mode,
  type, ownership, visibility, and explicit generic specialization checks.
- `collections.Map[K,V]` is opaque, move-only, restricted to `int`, `bool`, or
  `string` keys, and cannot be directly constructed as a struct literal.
- Accepted standard calls and function values retain a typed intrinsic identity
  in IR. Implemented standard functions also work through ordinary function
  values and indirect calls.
- `string` values are immutable valid UTF-8. `std/strings` implements byte
  length, Unicode scalar count, byte-offset search, split, join, integer and
  boolean formatting, and strict parsing with the signatures fixed by the v0.6
  contract.
- `find` returns the first byte offset, including `Some(0)` for an empty needle.
  Splitting on a non-empty separator preserves empty fields; an empty separator
  splits by Unicode scalar value and returns an empty slice for empty input.
- `parseInt` accepts only an optional leading `-` and ASCII digits. Empty input,
  whitespace, `+`, and overflow return `Err(errors.Error)` with
  `errors.Kind.InvalidData`. `parseBool` accepts exactly `true` or `false`.
- Owned string, slice, and error results use the compiler allocation accounting,
  cleanup, deterministic failure injection, and fatal malformed-runtime-string
  boundary.
- `std/os.args` returns an owned UTF-8 argument slice including the invocation
  name at index 0. `std/os.env` distinguishes a missing value from invalid input
  or data, and `std/os.exit` accepts process codes from 0 through 255.
- `std/io.readStdin` reads all stdin as valid UTF-8 while preserving embedded
  NUL bytes. `writeStdout` and `writeStderr` perform length-based exact writes;
  read, write, and flush failures return `errors.Error`.
- `mlg run <input> -- <program-args>` forwards arguments unchanged and propagates
  a generated program's numeric exit status. Direct and runner-based invocation
  use the same generated process ABI.
- `std/fs.readText` accepts only NUL-free paths and returns owned valid UTF-8 while
  preserving embedded NUL content. `writeText` creates or overwrites a file with
  exact length-based bytes. Open, read, write, and close failures return
  `errors.Error`; invalid file text returns `InvalidData`.
- `std/collections` implements opaque specialized `Map[K,V]` values with
  deterministic value/content hash and equality, owned insert/remove, call-scoped
  read/update callbacks, capacity-checked growth, and recursive cleanup.
- Replacing a map entry cleans up the incoming key and returns the old value;
  removing an entry cleans up its stored key and transfers value ownership.
- The multi-package `examples/projects/textstats` CLI composes arguments, file
  and stream I/O, UTF-8 text, and `Map[int,int]` with exhaustive `Result` matches.
  Its error-flow review keeps `?` outside v0.6 because early return cleanup,
  return-type compatibility, and process-exit policy require a joint decision.
- Normal P148-P152 acceptance programs finish with zero live allocations.
  Recoverable platform failures remain `Result` values; fatal runtime failures
  retain the no-unwind contract.

## Implemented v0.3 Function Values and Closures

The approved v0.3 surface adds first-class function values and owned closures:

- Function types use `func(int) int`; parameter modes are part of the type and
  the return type is required. A no-value function type ends in `unit`.
- Named functions and package-qualified named functions can be used as values.
- Closure literals use `func(value int) int { ... }`.
- Mutable closure types and literals use `func mut(int) int` and
  `func mut(value int) int { ... }`.
- Free local bindings are captured by owned value. Copy values are copied and
  non-copy values are moved into the closure environment.
- Plain closure captures are immutable. Mutable closures require exclusive
  access to call and can modify captures originating from mutable bindings.
- Nested closures propagate lexical free variables through enclosing owned
  environments. A borrowed non-copy outer capture cannot be moved again.
- Package-qualified named functions work as values, and public function types
  can cross package API boundaries.
- Borrowed captures, explicit capture lists, and recursive local closures are
  excluded from v0.3.

Function values are move-only. Indirect calls borrow the callable for the call:
plain function values use read access and mutable closure values use exclusive
access. Capturing environments are compiler-owned and cleaned up exactly once.

## Naming

The language name is Mallang.

```text
source extension: .mlg
user-facing command: mlg
build command: mlg build
run command: mlg run
check command: mlg check
version command: mlg --version
help command: mlg --help
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

## Non-goals for Published v0.6

- No goroutines.
- No interfaces.
- No user-visible lifetimes.
- No first-class borrowed references or statement-spanning borrows.
- No error-propagation operator.
- No garbage collector.
- No raw pointer or address-of syntax.

## Source Shape

```go
func main() {
    name := "kim"
    msg := greet(con name)
    print(msg)
}

func greet(con name string) string {
    return "hello " + name
}
```

The program entrypoint is exactly `func main()` in v0. `main` must not declare a
receiver, parameters, or a return type, and the name is reserved for the
entrypoint rather than receiver-qualified methods.

## Lexical Rules

- Identifiers use ASCII letters, digits, and `_`.
- Identifiers cannot start with a digit.
- Line comments start with `//`.
- String literals use double quotes.
- Integer literals are base-10 in v0.
- Arithmetic operators `+`, `-`, `*`, `/`, and `%` support `int` operands.
  Literal division or remainder by zero is rejected by `mlg check`; dynamic zero
  divisors fail with a Mallang runtime error in native code. Literal arithmetic
  overflow is rejected by `mlg check`; dynamic arithmetic overflow fails with a
  Mallang runtime error in native code.
- Equality operators `==` and `!=` support `int`, `bool`, and `string`. String
  equality compares contents and does not move the compared values.
- Bool operators `!`, `&&`, and `||` support `bool` operands. `&&` and `||`
  use short-circuit evaluation in the native backend.

Reserved words:

```text
package import pub func return if else for break continue range match case mut con true false struct type nil
```

`nil` is reserved so the compiler can produce a clear error instead of treating
it as an ordinary identifier.

Reserved built-in value names:

```text
print len append Some None Ok Err
```

These names cannot be used as top-level type or function declarations,
parameter or receiver names, local bindings, range bindings, or match payload
bindings in v0. Method and field names use dot-qualified namespaces and are not
part of this value namespace rule.

Top-level `type` declarations and non-method `func` declarations share a
declaration namespace in v0. A program cannot declare both `type User struct`
and `func User(...)` at top level. Concrete method names are scoped by receiver
type, so `func (con self User) User()` remains a receiver-qualified method.

## Bindings

Bindings are immutable by default.

```go
x := 10
mut y := 20
y = y + x
```

Rules:

- Reassignment requires a `mut` local binding.
- Redeclaring a binding in the same block is rejected.
- Shadowing is allowed only in a nested block or arm-local scope, including
  `if`, `for`, `range` bodies, and `match` arms.
- A move of a shadowed inner binding does not move the outer binding.
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

Fixed-size arrays use Go-like `[N]T` type syntax. v0 supports compile-time
integer lengths and element types that already work in the native backend.

```go
values := [3]int{1, 2, 3}
```

Array rules:

- `N` must be a non-negative integer literal.
- Array literals must provide exactly `N` elements.
- Every element must have the array element type.
- Arrays are move-only in v0, even when the element type is `Copy`.
- The first native layout is a C struct wrapper with a fixed `data[N]` field,
  not a raw C array, so array values can be assigned, moved, and passed through
  the existing value pipeline.
- Fixed-size array indexing as a value is supported for `Copy` elements.
- Fixed-size array `len` and mutable element assignment are supported for
  `Copy` and non-copy elements.
- Fixed-size array element borrow arguments are supported for `Copy` and
  non-copy elements.
- Fixed-size array indexing is also valid in read-only borrow contexts for
  non-copy elements, such as `print(users[i])` or `print(users[i].name)`.
- Fixed-size array element method receivers are supported for `con` and `mut`
  receiver modes.
- Slice type syntax `[]T` denotes an owned, move-only growable buffer. It is not
  a Go-style aliasing slice header. The first source-level slice surface supports
  slice literals, `len(slice)`, Copy-only `slice[i]` value access, and consuming
  `append(slice, item)` growth. Slice range supports the same Copy value
  iteration surface as arrays. Slice element borrow arguments extend the same
  local-rooted borrow surface as fixed-size arrays. Direct mutable slice element
  assignment is supported for `Copy` and non-copy element types. Same-field
  append reassignment is supported for owned slice field paths, including stable
  indexed field paths. Owned slice field paths can also be taken in owned value
  positions by leaving the source field as an empty slice. Read-only indexed
  expressions can inspect non-copy slice elements without moving them. Mutable
  range values, general partial field moves beyond slice field take, and
  non-copy element extraction remain reserved for later slices.

Fixed-size array indexing and length share the value-read surface with the first
owned slice implementation.

```go
value := values[i]
count := len(values)
values[i] = 5
show(con users[i])
rename(mut users[i].name)
users[i].name = "lee"
slice := []int{1, 2, 3}
sliceCount := len(slice)
sliceValue := slice[0]
for ; i < 3; values[slot] = i {
    slot = i
    i = i + 1
}
```

Indexing and length rules:

- `values[i]` is valid when `values` has fixed-size array type `[N]T` or slice
  type `[]T` and `i` has type `int`.
- `values[i]` yields a value only when `T` is `Copy`.
- `values[i]` may also be used in read-only borrow contexts when `T` is
  non-copy. This supports printing or reading Copy fields from indexed
  array/slice elements without moving the element.
- `con values[i]` and `mut values[i]` are valid as direct function call
  arguments even when `T` is non-copy. Field paths rooted in an indexed array or
  slice element, such as `con users[i].name`, are also valid borrow arguments.
- Mutable indexed element borrow arguments require the root binding to be `mut`.
- Borrow overlap checks treat indexed borrows from the same array root
  or slice root conservatively in v0. Distinct runtime indexes are not yet
  proven disjoint.
- `len(values)` returns `int` for fixed-size arrays and owned slices and does
  not move `values`.
- Integer literal indexes outside `0 <= i < N` are rejected by `mlg check`.
- Negative integer literal slice indexes are rejected by `mlg check`. Slice
  upper bounds are checked by generated native code because slice length is a
  runtime header value.
- Non-literal indexes are checked by generated native code before element access.
  An out-of-bounds runtime index terminates the program with a Mallang runtime
  error instead of performing unchecked C memory access.
- `len(slice)` and `slice[i]` may read either a local-rooted slice source, such
  as `values`, `bag.values`, or `matrix[i]`, or a computed owned slice. A
  computed cleanup value is held in a compiler-owned full-expression temporary
  through the final length or element read and then dropped.
- Slice element borrow arguments accept local-rooted sources such as
  `con values[i]`, `mut bag.values[i]`, or `con users[i].tags[j]`. A computed
  owned source is held in a full-expression temporary through the call.
- Indexed field assignment such as `users[i].name = expr` is valid for
  local-rooted fixed-size arrays and direct local slices when the root binding is
  mutable. Slice indexed field assignment uses the same negative literal and
  native runtime `mlg_len` bounds checks as slice element assignment.
- Indexed element assignment requires a local-rooted mutable array/slice source,
  such as `values[i] = expr` or `bag.values[i] = expr`.
- `values[i] = expr` can also be used as a Go-like `for` clause post target
  and follows the same mutable indexed-place rules.
- Indexed element assignment supports `Copy` and non-copy element types.
- For non-copy element types, the right-hand expression is owned and moves into
  the array or slice slot.
- Fixed-size array element assignment uses the same compile-time literal and
  native runtime bounds checks as array indexing. Slice element assignment
  rejects negative literal indexes at `mlg check` time and uses native runtime
  upper-bound checks against `mlg_len`.
- The assignment index is evaluated and bounds-checked before the right-hand
  expression is evaluated.
- The native backend lowers `for` clause conditions and post assignments that
  need temporary prelude statements, including fixed-size array `len(values)`
  conditions and indexed post expressions.
- In a three-clause `for`, `continue` skips the remaining body and then executes
  the post assignment before the next condition check.
- Slice literals use `[]T{...}` and produce owned heap-backed slices. Empty
  slices use a null data pointer with zero length and capacity.
- `append(values, item)` consumes the owned slice and the owned item, then
  returns a new owned slice. Updating a local therefore uses normal mutable
  reassignment, such as `values = append(values, item)`.
- Direct owned slice field paths can use same-field append reassignment, such
  as `bag.values = append(bag.values, item)` or
  `shelf.bag.values = append(shelf.bag.values, item)`, when the root binding is
  mutable.
- Owned slice field paths can also be used as append sources without same-field
  reassignment, such as `grown := append(bag.values, item)`. This is a
  compiler-owned take: the append result owns the consumed buffer, and the
  source field is reset to an empty slice before the owning struct is later
  dropped.
- Owned slice field paths can also be taken in ordinary owned value positions,
  such as `taken := bag.values` or `consume(bag.values)`. This uses the same
  compiler-owned take rule: the result or callee owns the consumed buffer, and
  the source field is reset to an empty slice.
- Indexed owned slice field paths can also use same-field append reassignment,
  such as `store.bags[i].values = append(store.bags[i].values, item)`, when the
  target and source paths match structurally and every index expression in the
  matched path is stable. Stable index expressions are side-effect-free
  expression forms such as literals, variables, field/index reads, unary
  expressions, and binary expressions over stable operands. Calls, `if`,
  `match`, and literals that allocate cleanup resources are not stable index
  expressions for this rule.
- `append` arguments do not take `con` or `mut` mode markers.
- Native slice literals and `append` use compiler-owned allocation. Allocation
  failure and allocation-size overflow terminate the program with a Mallang
  runtime error instead of unchecked C behavior. `append` also terminates on
  length overflow.
- The native ABI uses an internal header equivalent to `{ data, len, cap }` with
  compiler-owned allocation and cleanup.
- Copying a slice header is not a language operation. Assigning, passing, or
  returning an owned slice moves it, following the existing non-copy value rules.
- Fields with owned cleanup resources are supported in structs. Struct drop helpers
  recursively clean up owned strings, slices, ADTs, closures, and nested aggregates.

Implemented slice cleanup model:

- The backend emits internal drop helper shells for cleanup types.
- The typed IR can carry explicit internal drop statements.
- Automatic drop insertion covers straight-line owned cleanup parameters,
  locals, local reassignment, `if`/`match` branch-local cleanup roots,
  `if`/`match` outer cleanup root branch moves, expression branch cleanup,
  `for`/`range` body-local cleanup roots, and `for` init cleanup roots via a
  loop-exit cleanup trailer.

Full-expression temporary cleanup:

- A cleanup-valued computed expression used only for a read is materialized as
  a compiler-owned typed IR temporary. Direct local-rooted places remain
  borrowed and are not moved.
- When a read projects a field or indexed element from a computed owner, the
  compiler keeps the owning root temporary alive through the read and drops the
  root, not a copied projection.
- Call argument temporaries live through the call. This includes computed
  values passed with `con` or `mut`; the caller drops the temporary after the
  callee returns.
- Discarded cleanup-valued expression results are dropped at the end of the
  expression statement.
- `if` and `for` condition temporaries are dropped after each condition
  evaluation. `&&` and `||` create and clean up their right-hand temporaries
  only when short-circuit evaluation selects the right-hand side.
- Index and `len` source temporaries live through the final read. Index bounds
  guards run before element access, and normal-flow cleanup follows the access.
- A computed range source is owned by the loop and remains alive for every
  iteration. It is dropped once on normal exit, `break`, or an enclosing
  function return; `continue` keeps it alive for the next iteration.
- Return values are evaluated before local cleanup and then transferred to the
  caller. Full-expression temporaries not transferred by the return are
  cleaned up before returning.
- Mallang runtime guards use fatal no-unwind termination. A guard failure may
  terminate before pending cleanup runs; process termination, rather than stack
  unwinding, is the v0 runtime failure contract.

String runtime contract:

- `string` is one immutable, move-only source type. A static literal and a
  heap-owned buffer have the same value semantics and use the same parameter,
  return, local, field, enum payload, and closure capture rules.
- A string value contains a byte sequence and length. Equality compares length
  and bytes, and `print` writes exactly that byte sequence. Neither operation
  moves the string.
- Static and owned storage are compiler/runtime details. Source code cannot
  inspect a storage kind or address, and no separate source type or ownership
  syntax is introduced for heap-backed strings.
- Moving a string transfers its storage responsibility. Dropping a static
  string performs no deallocation; dropping an owned string releases its buffer
  exactly once on normal control flow.
- Aggregate drop helpers recursively apply this contract to string fields,
  variant payloads, and closure captures.
- Replacing a string evaluates the replacement first, evaluates the destination
  place once, drops the old value, and then stores the replacement. A `mut`
  parameter or mutable closure capture retains the replacement for its external
  owner instead of dropping it at the end of the call.
- Malformed storage, invalid data, allocation-size overflow, and allocation
  failure are fatal no-unwind runtime errors. Normal execution remains subject
  to exactly-once cleanup.
- v0.5 establishes the representation and cleanup contract. Source-level
  operations that allocate new string buffers are deferred to the v0.6 standard
  library surface.

Compiler-owned allocation contract:

- Slice buffers, closure environments, recursive enum nodes, and owned string
  buffers use one compiler runtime allocation boundary. Raw allocation handles,
  counters, and failure controls are not Mallang source values or APIs.
- A successful allocation that creates a new owned storage lifetime increments
  the live allocation count. Growing an existing allocation preserves that
  lifetime; growing an empty buffer creates one. Releasing a non-null owned
  allocation decrements the count exactly once.
- Deallocating null storage is a no-op. Deallocating non-null storage when no
  live allocation is recorded is an internal fatal accounting error.
- Internal test builds can deterministically fail a selected allocation attempt.
  The resulting diagnostic remains specific to the source operation, such as
  slice, closure, recursive enum, or string allocation failure. This test hook
  is not part of the source language or stable native ABI.
- A normal return from Mallang `main` must leave no compiler-owned live
  allocations. Fatal runtime failure remains no-unwind and does not promise
  cleanup or a zero live count before process termination.

Deferred slice and borrow rules:

- Mutable range values, by-reference iteration, borrowed slice views,
  first-class references, and sharing a backing buffer across multiple owned
  slice values are not part of v1.

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
- Struct values are cleanup-capable roots. Structs without cleanup fields lower
  to no-op drop helpers; structs with owned string, slice, ADT, closure, or nested
  cleanup fields recursively drop those fields at scope exit, reassignment, or
  owned parameter cleanup.
- Literal fields must name every declared field exactly once.
- Field access reads a field by name.
- Field assignment updates a field path rooted in a mutable local struct binding.
- Copy fields such as `int` and `bool` can be read as values.
- Non-copy fields such as `string` can be borrowed for calls like `print`, but
  moving a non-copy field out of a named struct is rejected until destructuring
  or partial-move semantics is designed. Owned slice fields are the v0 exception:
  moving `bag.values` takes the slice buffer and leaves `bag.values` empty so
  later struct cleanup remains safe. Moving a non-slice cleanup field such as
  `user.profile` remains rejected because it would leave the parent struct
  partially initialized without a general partial-move model.
- Field paths rooted in local bindings can be used as borrow arguments, such as
  `show(con user.name)` or `rename(mut user.profile.name)`.
- Mutable field borrow arguments require the root binding to be `mut`.
- Borrow conflict checks are field-aware within a single call: overlapping
  whole-struct/field borrows and same-field exclusive borrows are rejected,
  while disjoint mutable field borrows such as `mut pair.left`,
  `mut pair.right`, `mut user.name.first`, and `mut user.name.last` are allowed.
- `print` displays struct values as `Type{field: value, ...}` only when every
  field type is printable. Structs containing fixed-size arrays are rejected by
  `mlg check` until an array display format is designed.
- v0 rejects recursive by-value structs. This includes direct recursion and
  recursion through `Option`, `Result`, or fixed-size arrays. Recursive data
  modeling needs a future owned indirection or borrowed view design.
- v0 does not include struct pattern matching.

Methods use Go-like receiver declarations with Mallang's parameter mode syntax.

```go
func (con self User) age() int {
    return self.age
}

func (mut self Counter) inc() {
    self.value = self.value + 1
}

print(user.age())
```

Rules:

- The receiver must be a struct type in v0.
- Receiver modes are the same as parameter modes: owned, `con`, and `mut`.
- A method call implicitly passes the receiver according to the method
  declaration.
- `con` and `mut` method receivers may be direct local variables, local-rooted
  field paths, fixed-size array element paths such as `users[i].age()` and
  `counters[i].inc()`, or computed receiver values held in a call-scoped
  full-expression temporary.
- Returning or storing borrowed values is still unsupported, so methods with
  `con` receivers cannot return non-copy fields such as `string` by value.
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
con name T  // const/read-only borrow
mut name T  // mutable borrow
```

`con` and `mut` are the only borrow mode markers in v0. `in`, suffix
`name in T`, and suffix `name mut T` are not part of the language.

Call sites must make borrow mode explicit.

```go
readName(con user)
rename(mut user, "lee")
readField(con user.name)
renameField(mut user.profile.name)
consume(user)
```

`con expr` and `mut expr` are call argument mode prefixes only. They are not
general expressions and cannot be bound to locals, returned, stored in fields,
or used to create first-class reference values in v0.

Rules:

- Passing a non-copy value as `T` moves ownership into the callee.
- Passing `con T` creates a const/read-only borrow for the duration of the call.
- Passing `mut T` creates an exclusive mutable borrow for the duration of the call.
- Native code passes `con T` and `mut T` as hidden references. Inside the callee,
  reads use normal value syntax, and assignment through a `mut T` parameter
  updates the caller's local variable or field path.
- Borrow arguments may be local-rooted places or computed values. Computed
  cleanup values are owned by a caller-side full-expression temporary for the
  duration of the call.
- Non-copy borrowed parameters cannot be moved into owned locals, owned
  arguments, or return values. Copy borrowed parameters such as `int` and `bool`
  may still be copied out as values.
- Borrowed values cannot be stored in variables in v0.
- Borrowed values cannot be returned in v0.
- Non-`unit` functions must return on every path the v0 checker can prove.

## Built-in Statements

`print(expr)` is a statement-only built-in in v0.

Rules:

- `print(expr)` can appear as a direct expression statement.
- `print` cannot be used as a value, binding initializer, call argument,
  return expression, `if` branch value, or `match` arm value.
- `print` arguments do not take `con` or `mut` mode markers.
- The argument must have a printable type. v0 printable types are `int`,
  `bool`, `string`, built-in ADTs with printable payloads, and structs whose
  fields are printable.
- General expression statements may discard an owned cleanup value. The result
  is held in a compiler-owned full-expression temporary and dropped once at the
  end of the statement.

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

for {
    tick()
}

for mut i := 0; i < 3; i = i + 1 {
    print(i)
}

for ; i < 3; i = i + 1 {
    print(i)
}

for ; ; i = i + 1 {
    if i == 3 {
        break
    }
}
```

Rules:

- When present, the condition must have type `bool`.
- In the three-clause form, init is optional. When present, it is `name := expr`
  or `mut name := expr`.
- In the three-clause form, post is a single variable or field assignment.
- Bindings introduced by the init clause are scoped to the loop header and body.
- Bindings introduced inside the body do not leak outside the body.
- Moving a loop-persistent move-only value in a `for` condition, body, or post
  statement is rejected in v0. This includes outer bindings and three-clause
  init bindings. Move-only values created inside the body may still be moved
  within that iteration.
- `break` exits the nearest enclosing loop.
- `continue` skips to the next iteration of the nearest enclosing loop.
- `break` and `continue` are only valid inside loops.
- A `for` statement is not considered return-complete in v0, even when its
  condition is statically `true`.
- v0 does not yet include post declarations.

Range loops are a Go-like iteration form over fixed-size arrays and owned
slices.

```go
for i, value := range values {
    print(i)
    print(value)
}

for _, value := range values {
    print(value)
}

for i := range values {
    print(i)
}
```

Range rules:

- The range source must be a fixed-size array or owned slice.
- Range over slices accepts local-rooted and computed owned sources. An inline
  source such as `range []int{1, 2}` is held in a compiler-owned loop temporary
  and dropped on normal exit, `break`, or enclosing function return.
- The two-variable range form introduces immutable `int` index and immutable
  element value bindings scoped to the loop body.
- The one-variable range form introduces only the immutable `int` index binding.
- Range binding syntax intentionally has no `con` or `mut` marker in v1.
- Either range binding may be `_`. A blank binding is not added to the loop body
  scope.
- The element binding is allowed only when the element type is `Copy`; if the
  value binding is `_`, or the one-variable range form is used, no element copy
  is created and non-copy element arrays or slices may be ranged by index.
- The range source is read for iteration and is still usable after the loop.
- Assigning to the active range source binding inside the loop is rejected in
  v0.
- `break` and `continue` follow the same nearest-loop rules as other `for`
  forms.
- Mutable range variables and by-reference element iteration are not part of
  v1. Non-copy traversal uses the index-only form and accesses an element with
  indexed assignment or a direct call-scoped `con values[i]` / `mut values[i]`
  argument. Range over maps and strings is also deferred.

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
  `Err(value)` when the payload type itself is printable. ADTs containing
  fixed-size arrays are rejected by `mlg check` until an array display format is
  designed.
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

Normative v1 rule set:

- Every value has one owner unless it is `Copy`.
- Reading a `Copy` value does not change its ownership state. Assigning,
  passing, returning, or matching a non-copy value in an owned position
  transfers ownership and leaves the source moved.
- A moved binding or moved place cannot be read, borrowed, assigned through, or
  moved again. General partial field moves are rejected; the compiler-owned
  owned-slice field take is the documented exception that resets its source.
- `con expr` and `mut expr` are direct call argument modes, not expressions or
  reference values. Their borrow begins while preparing that call and ends when
  the callee returns.
- Any number of overlapping `con` borrows may coexist in one call. A `mut`
  borrow is exclusive and conflicts with every overlapping `con` or `mut`
  place. Disjoint struct fields may be borrowed independently; indexed places
  with the same root overlap conservatively.
- A borrowed non-copy value cannot be moved into an owned local or argument,
  returned, stored, or captured by a closure. A borrowed `Copy` value may be
  copied out.
- Mutation requires a mutable binding or a `mut` parameter/receiver. After a
  call-scoped mutable borrow ends, the caller still owns the possibly replaced
  value.
- Overwrite evaluates the replacement before dropping the old destination,
  evaluates a side-effecting destination place once, and stores the new owner
  after the drop.
- At a control-flow merge, a binding moved on any reachable incoming path is
  unavailable afterward. A loop-persistent move-only binding cannot be moved by
  a condition, body, or post path that may execute again; iteration-local values
  may be moved within that iteration.
- A return expression is evaluated into caller-owned storage before remaining
  callee locals are dropped. Returning a non-copy value transfers it to the
  caller.
- Range value bindings copy only `Copy` elements. Non-copy arrays and slices use
  index-only range and direct indexed `con`/`mut` call access; this does not
  create a statement-spanning borrow.
- First-class references, borrowed returns, user-visible lifetimes, and
  statement-spanning borrows are not part of v1.

These rules preserve no dangling references, no use-after-free, exactly-once
normal-flow ownership cleanup, and no data races within single-threaded v1
execution without exposing pointer or lifetime syntax.

## Backend Strategy

Initial native path:

```text
source -> tokens -> AST -> typed AST -> ownership checked IR -> C -> clang -> native binary
```

The C backend is an implementation tool, not the language's semantic model.
