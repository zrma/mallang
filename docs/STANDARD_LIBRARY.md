# Mallang Standard Library Reference

This document describes the implemented v0.6 standard-library surface. Standard
packages ship with the compiler and use the compiler version; they are not
independently versioned dependencies.

## Imports and ownership

Standard packages use ordinary imports:

```mlg
import "std/errors"
import "std/fs"
import "std/io"
import "std/os"
import "std/strings"
import "std/collections"
```

The `std/...` namespace and project name `std` are reserved. Standard calls use
the same explicit generic arguments and `con`/`mut` argument modes as project
functions. No standard API exposes a native pointer, handle, allocator, or
borrowed return value.

Recoverable failures return `Result[..., errors.Error]`. Allocation failure,
capacity overflow, malformed compiler-owned storage, and invalid `os.exit` codes
are fatal runtime errors and do not unwind.

## `std/errors`

```mlg
type Kind enum {
    NotFound
    PermissionDenied
    AlreadyExists
    InvalidInput
    InvalidData
    Interrupted
    Other
}

type Error struct {
    kind Kind
    message string
}
```

`Kind` is a platform-independent Copy value. `Error` owns its UTF-8 message.
Platform-specific numeric codes and native handles are not exposed.

## `std/strings`

```mlg
strings.byteLen(con text string) int
strings.scalarCount(con text string) int
strings.contains(con text string, con needle string) bool
strings.find(con text string, con needle string) Option[int]
strings.split(con text string, con separator string) []string
strings.join(con parts []string, con separator string) string
strings.fromInt(value int) string
strings.parseInt(con text string) Result[int, errors.Error]
strings.fromBool(value bool) string
strings.parseBool(con text string) Result[bool, errors.Error]
```

- `string` is immutable valid UTF-8 text.
- `byteLen` counts bytes; `scalarCount` counts Unicode scalar values, not
  grapheme clusters.
- `find` returns the first byte offset. An empty needle returns `Some(0)`.
- A non-empty split separator preserves leading, trailing, and consecutive
  empty fields. An empty separator splits by scalar value; empty input produces
  an empty slice.
- `split`, `join`, and formatting results are owned.
- `fromInt` uses canonical base 10. `parseInt` accepts only an optional leading
  `-` and ASCII digits; empty text, whitespace, `+`, and overflow return
  `Err(InvalidData)`.
- `parseBool` accepts exactly `true` or `false`.

## `std/os`

```mlg
os.args() Result[[]string, errors.Error]
os.env(con name string) Result[Option[string], errors.Error]
os.exit(code int)
```

- `args` returns owned UTF-8 arguments with the invocation name at index 0.
- `mlg run <input> -- <program-args>` preserves argument order and bytes before
  UTF-8 validation by the generated program.
- `env` returns `Ok(None)` when the variable is missing. A name containing NUL
  returns `Err(InvalidInput)`; invalid UTF-8 returns `Err(InvalidData)`.
- `exit` accepts `0..255` and terminates immediately without Mallang cleanup.

## `std/io`

```mlg
io.readStdin() Result[string, errors.Error]
io.writeStdout(con text string) Result[unit, errors.Error]
io.writeStderr(con text string) Result[unit, errors.Error]
```

`readStdin` reads to EOF, preserves embedded NUL bytes, validates UTF-8, and
returns owned text. Writes are length-based exact writes. Read, write, and flush
failures are recoverable `Error` values.

## `std/fs`

```mlg
fs.readText(con path string) Result[string, errors.Error]
fs.writeText(con path string, con text string) Result[unit, errors.Error]
```

- Paths must not contain NUL.
- `readText` returns owned valid UTF-8 and preserves embedded NUL content.
- `writeText` creates or overwrites the target and writes exactly the supplied
  byte length. It is not an atomic-replace or append operation.
- Open, read, write, short-write, and close failures return `errors.Error`.

## `std/collections`

```mlg
collections.newMap[K, V]() collections.Map[K, V]
collections.count[K, V](con map collections.Map[K, V]) int
collections.insert[K, V](mut map collections.Map[K, V], key K, value V) Option[V]
collections.with[K, V](
    con map collections.Map[K, V],
    con key K,
    con visit func(con V) unit
) bool
collections.update[K, V](
    mut map collections.Map[K, V],
    con key K,
    con edit func(mut V) unit
) bool
collections.remove[K, V](
    mut map collections.Map[K, V],
    con key K
) Option[V]
```

- `Map[K,V]` is opaque and move-only. It cannot be constructed as a struct or
  printed.
- `K` must be concrete `int`, `bool`, or `string`. Hash and equality use values
  and UTF-8 byte content, never storage addresses.
- `insert` moves key and value into the map. On replacement it cleans up the
  incoming key and returns the old value as `Some`.
- `with` and `update` invoke the callback exactly once when the key exists and
  return `true`; otherwise they do not invoke it and return `false`. The value
  borrow cannot outlive the callback.
- `remove` cleans up the stored key and transfers value ownership to the caller.
- Dropping a map drops every remaining key and value exactly once.
- Iteration, stable order, borrowed lookup returns, implicit clone, custom hash,
  and shared ownership are not part of v0.6.

## Error flow

v0.6 uses exhaustive `match`; it has no `?`, exception, implicit process exit,
or stack unwinding:

```mlg
match fs.readText(con path) {
    case Ok(text) {
        print(strings.scalarCount(con text))
    }
    case Err(error) {
        print(error.kind)
    }
}
```

The multi-package `examples/projects/textstats` project demonstrates arguments,
file I/O, text operations, `Map`, stderr, and process exit behavior.

## Supported native acceptance

The v0.6 acceptance target is macOS arm64 and Linux x86_64 through the generated
C11 host-runtime path. Windows, cross compilation, binary I/O, networking,
async I/O, long-lived handles, and public FFI are not declared supported.
