# Open Questions: v0.6 Standard Library

상태: approved (2026-07-15)

이 문서는 v0.6에서 구현할 public API와 compatibility boundary를 정리한다.
아래 표기는 실제 source spelling이다. 예를 들어 `import "std/os"` 뒤에는
`os.args()`를 호출한다.

## Q1. Standard package namespace and resolution

추천: implicit global builtin을 늘리지 않고 다음 compiler-owned package를 기존 import
syntax로 제공한다.

```mlg
import "std/errors"
import "std/fs"
import "std/io"
import "std/os"
import "std/strings"
import "std/collections"
```

- `std/...` package는 compiler distribution에 포함되고 compiler version과 함께
  versioning한다.
- Project source package나 dependency가 `std/...` namespace를 shadow할 수 없다.
- Project name `std`는 예약한다. 이는 v1 이전 compatibility decision이다.
- Standard import는 project mode와 standalone `.mlg` mode에서 같은 방식으로 동작한다.
- Compiler는 public signature와 typed intrinsic identity를 package graph에 합성한다.
  호출은 일반 visibility, type, ownership 검사를 통과한 뒤에만 runtime intrinsic으로
  lowering한다.
- User source에는 `extern`, raw C symbol, `unsafe`나 runtime internal name을 노출하지
  않는다.

대안은 standard source tree를 project마다 복사하거나 모든 API를 global builtin으로
만드는 것이다. 전자는 compiler/library version drift를 만들고 후자는 기존 package
model을 우회하므로 채택하지 않는다.

## Q2. Entrypoint and process state

추천: 기존 `func main()` contract를 유지하고 process state를 `std/os`에서 읽는다.

```mlg
os.args() Result[[]string, errors.Error]
os.env(con name string) Result[Option[string], errors.Error]
os.exit(code int)
```

- `args()` 결과의 index 0은 invocation name이고 나머지는 user argument다.
- `mlg run <input> -- <program-args>`는 separator 뒤 argument를 generated program에
  byte-for-byte 순서대로 전달한다. Built binary를 직접 실행하는 경우도 같은
  `os.args()` contract를 사용한다.
- Argument나 environment value가 valid UTF-8이 아니면 `Err(InvalidData)`를 반환한다.
  따라서 UTF-8을 엄격히 보장하면서 `args() []string` 또는
  `env(...) Option[string]`만 반환하는 초안은 채택하지 않는다.
- Missing environment variable은 `Ok(None)`이고 present value는 `Ok(Some(value))`다.
- Environment name에 NUL이 있으면 platform API에 truncated name을 넘기지 않고
  `Err(InvalidInput)`을 반환한다.
- `os.exit`는 `0..255` code만 허용하는 immediate no-unwind process termination이다.
  범위 밖 code는 programmer error로 fatal runtime diagnostic을 낸다.
- `os.exit`는 Mallang drop을 실행하지 않는다. Recoverable operation failure는 먼저
  `Result`로 처리하고 process boundary에서만 exit code로 바꾼다.
- `func main() Result[...]`와 parameter-bearing `main`은 v0.6에 추가하지 않는다.

## Q3. String, conversion and Unicode semantics

추천: `string`은 immutable valid UTF-8 text이며 byte offset과 Unicode scalar count를
API 이름으로 구분한다.

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

- `scalarCount`는 grapheme cluster가 아니라 Unicode scalar value 개수를 센다.
- `find`는 첫 left-to-right match의 byte offset을 반환하고 empty needle은 `Some(0)`이다.
  Byte offset을 scalar index로 해석하지 않는다.
- Non-empty separator의 `split`은 leading/trailing/consecutive empty field를 보존한다.
  Empty separator는 scalar마다 한 element를 만들고 empty input에는 empty slice를
  반환한다.
- `split`은 owned slice와 owned element string을 반환하고 `join`/conversion은 owned
  string을 반환한다. Borrowed input보다 오래 사는 view나 substring reference는 없다.
- `fromInt`는 canonical base-10이고 `parseInt`는 optional leading `-`와 ASCII digit만
  허용한다. Whitespace, `+`, empty input과 overflow는 `Err(InvalidData)`다.
- `parseBool`은 정확히 `true` 또는 `false`만 허용한다.
- Invalid UTF-8은 platform input에서 `Err(InvalidData)`다. Compiler/runtime가 만든
  malformed string은 recoverable input error가 아니라 existing fatal invariant failure다.
- Existing global `len`은 array/slice 전용으로 유지한다. String indexing, grapheme
  cluster API, normalization과 locale-aware case conversion은 v0.6 범위가 아니다.

## Q4. File and stream I/O

추천: text file과 standard stream의 최소 surface를 모두 v0.6에 포함한다.

```mlg
fs.readText(con path string) Result[string, errors.Error]
fs.writeText(con path string, con text string) Result[unit, errors.Error]

io.readStdin() Result[string, errors.Error]
io.writeStdout(con text string) Result[unit, errors.Error]
io.writeStderr(con text string) Result[unit, errors.Error]
```

- File content와 stdin은 valid UTF-8만 `Ok(string)`으로 반환한다.
- Path에 NUL이 있으면 truncated path를 사용하지 않고 `Err(InvalidInput)`을 반환한다.
  Text content의 embedded NUL은 length-based I/O로 보존한다.
- Read/write/open/close failure는 standard `Error`로 보존한다. Short write와 close
  failure도 성공으로 숨기지 않는다.
- `writeText`는 overwrite/create semantics다. Atomic replace와 append는 별도 API가
  필요해질 때 추가한다.
- `print`는 기존 간단한 value output builtin으로 유지한다. Standard stream API는
  exact bytes와 recoverable write failure가 필요한 code를 위한 것이다.
- Binary buffer, file handle, seek와 long-lived stream object는 v0.6에 추가하지 않는다.

## Q5. Standard error value

추천: 모든 recoverable standard-library failure는 `std/errors`의 owned value를 쓴다.

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

- `kind`는 platform-independent category이고 `message`는 owned UTF-8 설명이다.
- Platform errno, native handle, C pointer와 platform-specific numeric code를 source에
  노출하지 않는다.
- Unknown platform failure는 `Other`로 mapping한다. Native message가 valid UTF-8이면
  보존하고 아니면 stable ASCII fallback message를 사용한다.
- Out-of-memory, allocation size overflow와 malformed compiler-owned storage는
  recoverable `Error`가 아니라 v0.5 fatal no-unwind runtime failure로 유지한다.

## Q6. Error propagation syntax

추천: v0.6에는 `?`, exception, implicit process exit를 추가하지 않는다.

- Standard API와 reference CLI는 existing exhaustive `match`로 error flow를 작성한다.
- P152에서 실제 multi-module CLI의 반복 boilerplate를 측정한다.
- Boilerplate가 v1 usability를 막는 근거가 생기면 v0.8 hardening 전 별도 language
  decision gate를 열 수 있다.
- 승인 없이 `?`를 parser sugar나 hidden early return으로 추가하지 않는다.

이 선택은 propagation syntax를 영구 금지하는 결정이 아니라 v0.6 API/runtime를 먼저
검증하겠다는 compatibility boundary다.

## Q7. Owned key-value collection

추천: `std/collections`의 opaque `Map[K, V]`를 compiler-owned move-only type으로
제공한다.

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

- v0.6 key type은 concrete `int`, `bool`, `string`만 허용한다. Trait/constraint나
  user-defined hashing은 추가하지 않는다.
- `insert`는 key/value를 map으로 이동한다. Existing key가 있으면 incoming key는
  정리하고 old value를 `Some`으로 반환한다.
- Key equality/hash는 `int`/`bool` value와 `string` byte content를 사용하고 runtime
  storage address나 randomized user-observable iteration order에 의존하지 않는다.
- `with`/`update`는 key가 있을 때 callback을 call-scoped로 한 번 호출하고 `true`를
  반환한다. 없으면 callback을 호출하지 않고 `false`를 반환한다. Callback과 value
  borrow를 저장하거나 호출 뒤 유지하지 않는다.
- `remove`는 stored key를 정리하고 value ownership을 caller에게 반환한다.
- Map drop은 남은 key/value를 각각 정확히 한 번 정리한다.
- Borrowed return, iterator, implicit clone, shared ownership과 stable iteration order는
  제공하지 않는다.
- Allocation failure와 capacity overflow는 existing fatal runtime failure contract를
  사용한다.

`Map`을 ordinary user struct로 구현하면 opaque storage를 표현하려고 pointer wrapper나
unsafe surface가 필요하므로 채택하지 않는다. Public 함수 형태를 사용해 built-in
receiver method와 independently generic method를 새로 열지 않는다.

## Q8. Platform and native acceptance matrix

추천: v0.6 standard runtime은 macOS arm64와 Linux x86_64의 C11/host runtime 경로를
동시에 유지한다.

- Local supported-host native smoke와 Ubuntu CI native smoke를 모두 필수로 둔다.
- Standard-library reference CLI는 strict C warning gate와 ASan/UBSan gate를 통과한다.
- Arguments, environment, UTF-8 rejection, missing file, permission/write failure와 non-zero
  process exit를 platform-independent assertion으로 검증한다.
- Platform error message 원문은 다를 수 있으므로 acceptance는 `Kind`, success/failure와
  exit behavior를 기준으로 한다.
- Windows, cross compilation과 release artifact matrix는 v0.7 tooling/platform decision
  gate까지 지원 대상으로 선언하지 않는다.

## 승인 기록

Q1-Q8 추천안은 2026-07-15 사용자 승인을 받았다. `import`와 `func main()`을 유지하고,
`?`는 v0.6에 넣지 않은 채 P152에서 Rust-style propagation 필요성을 재평가한다.
Implementation은 P147부터 P153까지 순서대로 진행한다.
