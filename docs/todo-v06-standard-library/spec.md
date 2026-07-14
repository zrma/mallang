# Spec: v0.6-standard-library

상태: implementation in progress (P147 complete; P148 next)

## 목표

- 실제 native CLI program에 필요한 arguments, environment, text, file/stream I/O와
  key-value collection을 ownership-safe standard package로 제공한다.
- Recoverable platform failure를 `Result`로 표현하고 compiler fatal runtime guard와
  분리한다.
- v0.5의 move, call-scoped borrow, cleanup과 allocation contract를 모든 standard API에
  적용한다.
- Project source와 standalone source에서 같은 `std/...` import contract를 제공한다.

## Public surface recommendation

- Package: `std/errors`, `std/fs`, `std/io`, `std/os`, `std/strings`,
  `std/collections`.
- Entrypoint: existing `func main()` 유지.
- Process input: UTF-8 validation 때문에 `args`와 `env`도 `Result`를 반환한다.
- Text: immutable valid UTF-8, named byte offset와 Unicode scalar operations.
- I/O: file과 standard stream failure를 `Result[T, errors.Error]`로 반환한다.
- Error flow: v0.6은 explicit exhaustive `match`; `?` syntax는 근거가 생길 때 별도
  decision gate.
- Collection: compiler-owned opaque `collections.Map[K, V]`, key는 `int`/`bool`/`string`,
  non-Copy access는 call-scoped callback.

Exact signatures와 edge semantics는 `open-questions.md` Q1-Q8이 소유한다. 이 문서는
승인된 implementation contract이고 아직 implemented API로 해석하지 않는다.

## Compiler and runtime ownership boundary

현재 compiler pipeline은 project source만 package graph에 넣고, linker 뒤 semantic
signature collection과 typed IR/C backend를 재사용한다. v0.6은 이 구조를 다음처럼
확장한다.

1. Compiler-owned standard registry가 package/type/function public signature와 stable
   intrinsic identity를 정의한다.
2. Package graph가 `std/...` import를 registry에서 resolve한다. Local source package는
   이 namespace를 제공하거나 shadow할 수 없다.
3. Linker는 standard reference도 ordinary qualified symbol처럼 resolve하고 call-site
   span을 보존한다.
4. Semantic checker는 registry signature에 ordinary argument mode, type와 ownership
   rule을 적용한다. Runtime implementation 여부가 type checking을 우회하지 않는다.
5. Generic standard function은 explicit concrete type argument로 static specialization한다.
   Backend에 unresolved generic parameter가 남지 않는다.
6. Typed IR은 string operation, process/I/O와 map operation을 untyped C symbol string이
   아니라 typed intrinsic identity로 보존한다.
7. C backend/runtime만 platform API, opaque map storage와 native pointer를 다룬다. User
   source에는 raw handle, allocator나 unsafe escape hatch가 없다.

Standard declaration을 user-visible source stub으로 위장하거나 빈 function body로
lowering하지 않는다. Missing runtime implementation, signature mismatch와 unsupported
intrinsic type은 source error가 아니라 compiler invariant failure다.

## Feasibility findings

### Existing paths that can be reused

- Import qualifier, visibility와 internal symbol linking.
- Explicit generic function specialization and function-typed callback signatures.
- `con`/`mut` direct-call argument checking and full-expression borrow temporaries.
- `Option`/`Result`, generic ADT payload ownership and exhaustive `match`.
- Static/owned `string` representation, common compiler allocation accounting and cleanup.
- Strict generated C, native output, allocation-failure and ASan/UBSan harness.

### Required compiler extensions

- Package graph에 source file이 없는 reserved standard package를 합성하는 registry hook.
- Standalone compilation에도 standard import linking을 적용하는 shared compilation world.
- Synthetic standard signature의 generic specialization과 source-facing diagnostic name 복원.
- Typed intrinsic call target과 demand-driven C runtime emission.
- `collections.Map[K, V]` semantic `Type`, cleanup classification, IR/C opaque layout and drop.
- Generated C `main`에서 `argc`/`argv`를 안전한 process runtime으로 전달하는 internal ABI.
- `mlg run <input> -- <program-args>` argument forwarding과 direct binary invocation parity.
- UTF-8 validator와 platform error category mapping.

### Rejected shortcuts

- Global builtin expansion: package/visibility model을 우회한다.
- Project마다 standard source copy: compiler/library version drift를 만든다.
- `args() []string`와 `env(...) Option[string]`: invalid UTF-8 failure를 표현할 수 없다.
- Stream I/O 후속 연기: `docs/V1_ROADMAP.md`의 v0.6 범위를 축소한다.
- Ordinary user struct `Map`: opaque native storage를 source pointer 없이 표현할 수 없다.
- Non-Copy map lookup return: first-class reference나 implicit clone이 필요하다.
- Untyped backend symbol matching: semantic signature와 runtime body drift를 검출하지 못한다.

## Implementation order

### P147: Standard Package Registry and Intrinsic ABI

상태: complete (2026-07-15)

- Reserved `std/...` package registry와 exact import resolution을 추가한다.
- Project/standalone compilation을 shared standard-aware linking path로 연결한다.
- Standard public type/function signature, generic specialization과 typed intrinsic identity를
  semantic/IR에 보존한다.
- Unknown standard package, shadow attempt, wrong arity/mode/type와 internal-name access를
  source diagnostic으로 고정한다.

### P148: UTF-8 Text and Standard Error

상태: next

- `errors.Kind`/`errors.Error` owned value와 platform-independent error mapping을 추가한다.
- `strings` byte/scalar/search/split/join/conversion API를 구현한다.
- Every owned string/slice result를 allocation accounting, cleanup와 failure injection에
  연결한다.
- Invalid UTF-8, parse failure, empty split separator와 non-Copy result cleanup을 검증한다.

### P149: Process and Stream I/O

- Generated C main internal ABI에 arguments를 연결하고 `os.args`, `os.env`, `os.exit`를
  구현한다.
- `mlg run`의 `--` 뒤 argument를 generated binary에 그대로 전달한다.
- `io.readStdin`, `io.writeStdout`, `io.writeStderr`를 recoverable `Result` API로 구현한다.
- UTF-8 rejection, missing env, short/failing stream write와 exit code behavior를 검증한다.

### P150: File I/O

- `fs.readText`와 `fs.writeText`의 owned result/error cleanup을 구현한다.
- Not found, permission, invalid UTF-8, short write와 close failure mapping을 검증한다.
- Successful and failing operation을 strict C와 sanitizer native harness에 연결한다.

### P151: Owned Map

- Opaque specialized `Map[K, V]` layout, hash/equality, growth와 drop을 구현한다.
- `newMap`, `count`, `insert`, `with`, `update`, `remove`를 typed intrinsic으로 연결한다.
- Copy/non-Copy key/value, replacement/removal/callback와 allocation failure cleanup을
  검증한다.

### P152: Reference CLI and Error Flow Review

- Arguments로 input/output을 받고 file을 읽어 text/map transformation 뒤 file 또는 stdout에
  쓰는 multi-module CLI를 Mallang으로 작성한다.
- Expected failure를 explicit `match`, stderr와 non-zero exit로 처리한다.
- Repeated propagation boilerplate를 기록하고 `?` decision gate가 실제로 필요한지 판정한다.

### P153: v0.6 Acceptance and Documentation

- Local supported-host와 Ubuntu CI native acceptance를 닫는다.
- Standard API reference, `SPEC.md`, README, roadmap와 handoff를 implementation과 동기화한다.
- Strict C, allocation accounting/failure injection와 full generated C sanitizer sweep을
  통과한다.
- v0.6 completion evidence와 v0.7 decision gate를 작성한다.

## Compatibility contract

- Q1-Q8 승인 뒤 `std/...` import path, public type/function names와 documented semantics는
  v0.9 freeze 전까지 compatibility review 대상이다.
- Existing global builtin과 standalone source behavior는 명시적으로 바꾸지 않는다.
- Project name `std` 예약은 승인 시점부터 source compatibility break로 기록한다.
- Standard package version은 compiler version과 같고 independent package upgrade는 v0.6에
  없다.
- Platform-dependent message text는 stable contract가 아니다. `errors.Kind`, success/failure,
  ownership effect와 exit behavior가 contract다.

## Acceptance matrix

| Surface | Positive evidence | Rejection/failure evidence |
| --- | --- | --- |
| package | project/standalone `std/...` imports | unknown package, shadow, internal name |
| process | direct/`mlg run --` args와 env values | invalid UTF-8, NUL env name, invalid exit code |
| strings | byte/scalar/search/split/join/convert output | invalid UTF-8 and parse error |
| streams | stdin/stdout/stderr exact text | read/write failure and non-zero process behavior |
| files | read-transform-write CLI | not found, permission, invalid UTF-8, write/close failure |
| map | insert/read/update/remove with non-Copy values | invalid key type, borrow conflict, allocation failure |
| memory | zero live allocations on normal paths | deterministic failure diagnostics, no sanitizer finding |
| platform | supported-host and Ubuntu CI native smoke | platform text excluded from stable assertion |

## Excluded from v0.6

- `?`, exception, implicit process exit and stack unwinding
- Raw pointer, FFI, native handle and user allocator
- Binary buffer, long-lived file/stream handle, seek and async I/O
- String indexing, borrowed substring/view, grapheme/normalization/locale API
- User-defined map hash/equality, iterator and stable iteration order
- Shared ownership, first-class reference and borrowed return
- Windows and cross-compilation support declaration

## Decision record

`open-questions.md` Q1-Q8은 2026-07-15 사용자 승인을 받아 P146을 닫았다. `import`와
`func main()`을 유지하고, `?`는 v0.6에서 제외한 채 P152에서 Rust-style propagation
필요성을 재평가한다. P147-P153이 v0.6 execution order다. 승인된 contract를 바꾸려면
새 compatibility decision gate가 필요하다.
