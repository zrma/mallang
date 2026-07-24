# Mallang v1.1 Streaming Text I/O

Status: complete; released as v1.1.0 on 2026-07-17

## Goal

Add bounded-memory UTF-8 line processing for external Mallang programs without
exposing a native file handle or changing the v1 language contract.

## Public API

```mlg
fs.forEachLine[C, S](
    con path string,
    con context C,
    mut state S,
    con visit func(con C, mut S, int, con string) unit
) Result[unit, errors.Error]
```

The runtime opens the file once. `visit` runs synchronously for every line with
borrowed context, mutable borrowed state, a one-based line number, and borrowed
valid UTF-8 line text. The line excludes LF, preserves CR and embedded NUL, and
cannot escape the callback. A terminal LF does not produce a phantom line.
Callbacks completed before a later read or UTF-8 error remain observable; the
operation does not roll their state changes back.

Peak runtime storage is bounded by the longest line plus a fixed input buffer.
Open, read, invalid-data and close failures return `errors.Error`.

## Compatibility

- Release class: compatible minor `1.1.0`.
- Existing v1.0 source and behavior: unchanged.
- Affected rules: `V1-OWN-004`, `V1-OWN-006`, `V1-OWN-010`, `V1-STD-001`,
  `V1-STD-002`, `V1-STD-003`, `V1-STD-005`, and `V1-STD-008`.
- No syntax, keyword, public handle, borrowed return, native ABI, or platform
  error number is added.

## Acceptance

- [x] typed intrinsic and generic `C`/`S` specialization
- [x] function-value thunk and native callback ABI
- [x] deterministic line and UTF-8 semantics
- [x] embedded NUL and longest-line allocation behavior
- [x] recoverable open/read/close failures
- [x] strict generated C, ASan/UBSan and zero live-allocation checks
- [x] canonical Mallang gate
- [x] release-binary and clean installed-artifact Mallang gates
- [x] published v1.0.0 upgrade, rollback and re-upgrade compatibility rehearsal
- [x] macOS arm64 and Linux x86_64 release artifact gates
- [x] signed `v1.1.0` tag, GitHub Release and clean installer smoke
