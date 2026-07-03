# Mallang v0 Release Candidate Notes

## Status

This is the local v0 release-candidate snapshot for Mallang.

Remote publication is intentionally gated on explicit user approval because it
moves the `main` bookmark and pushes to GitHub.

## Language Surface

- Go-like syntax with `func`, `type Name struct`, `if`, `for`, `range`, and
  receiver methods.
- Canonical borrow markers are prefix-only `con` and `mut`.
- `func main()` is the only entrypoint shape.
- Functional value style includes `if` expressions, exhaustive `match`,
  built-in `Option[T]`, built-in `Result[T, E]`, and pipeline call sugar.
- Arrays use `[N]T`; owned slices use `[]T`.

## Safety Model

- Move/use-after-move checks for non-copy values.
- Same-call shared/mutable borrow conflict checks, including nested fields and
  indexed places.
- Borrow duration is limited to a single call in v0.
- Runtime guards cover integer division/remainder, checked integer arithmetic,
  array/slice bounds, allocation-size overflow, and allocation failure.
- Owned slices and slice fields are cleaned up by generated C drop helpers.

## Native Backend

- `mlg build` lowers typed IR to generated C and compiles a native binary.
- `con`/`mut` parameters use a hidden-reference C ABI.
- Generated C is gated by strict warnings for every checked-in successful
  example built by `scripts/check.sh`.
- Generated C memory/lifetime behavior is covered by focused sanitizer smoke and
  an explicit deep sanitizer sweep.

## CLI

- `mlg lex <source-file>`
- `mlg parse <source-file>`
- `mlg check <source-file>`
- `mlg ir <source-file>`
- `mlg build <source-file> -o <binary>`
- `mlg run <source-file>`
- `mlg --version`
- `mlg --help`

## Verification

Primary local gate:

```sh
scripts/verify-v0-rc.sh
```

Fast local rerun when generated C sanitizer artifacts are not required:

```sh
scripts/verify-v0-rc.sh --skip-deep-sanitizers
```

Publish-readiness verification without changing the jj description, moving
bookmarks, or pushing:

```sh
scripts/finalize-and-push.sh --verify-only
```

Approval-gated finalizer dry run that writes the final jj description and runs
remote freshness checks but does not move bookmarks or push:

```sh
scripts/finalize-and-push.sh --message "test: publish v0 release candidate" --no-push
```

## Deferred Beyond v0

- First-class borrowed references.
- Statement-spanning borrow lifetimes.
- General non-slice partial moves from fields.
- Mutable/by-reference range value syntax.
- Method values, interfaces, and dynamic dispatch.
- Modules/packages.
- Closures and higher-order functions.
- C interop boundary.
- LLVM or Cranelift backend.

## Publish

After explicit user approval:

```sh
scripts/finalize-and-push.sh --message "test: publish v0 release candidate"
```

The finalizer fetches `origin` before the expensive local verification and
again before moving the bookmark, prefers Homebrew Git when available, and
fails if `main@origin` no longer matches the local `main` base. After pushing,
it fetches again and verifies `main@origin` points at the published commit.
