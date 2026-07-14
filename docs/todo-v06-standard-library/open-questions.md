# Open Questions: v0.6 Standard Library

상태: recommendation draft; user approval required

## Q1. Standard package namespace

추천: implicit builtin 대신 `std.os`, `std.fs`, `std.strings`, `std.collections` package를 기존
import syntax로 사용한다.

## Q2. Entrypoint와 process state

추천: `func main()`을 유지하고 `std.os.args() []string`,
`std.os.env(con name string) Option[string]`으로 arguments와 environment를 읽는다.

## Q3. String and Unicode semantics

추천: `string`은 immutable UTF-8 text다. File/environment input은 invalid UTF-8을 `Error`로
거부한다. Existing `len(string)`은 이번 milestone에서 열지 않고, `std.strings.byteLen`,
`find`의 byte offset과 Unicode scalar operation을 이름으로 구분한다.

## Q4. File and stream I/O

추천: `std.fs.readText(con path string) Result[string, Error]`,
`writeText(con path string, con text string) Result[unit, Error]`를 최소 surface로 하고
stdin/stdout stream 확장은 후속 slice로 둔다.

## Q5. Standard error value

추천: standard `Error`는 owned code/message value이고 I/O API는 `Result[T, Error]`를 반환한다.
Platform errno나 raw handle은 source에 노출하지 않는다.

## Q6. Error propagation syntax

추천: 먼저 explicit exhaustive `match`로 standard API를 구현하고 검증한다. `?` 같은 propagation
syntax는 실제 반복 boilerplate가 확인된 뒤 별도 compatibility decision으로 승인한다.

## Q7. Owned key-value collection

추천: `std.collections.Map[K, V]`를 owned move-only value로 제공하고 v0.6 key는 `int`, `bool`,
`string`으로 제한한다. Non-Copy lookup은 borrowed return이나 implicit clone 대신
call-scoped callback으로 읽고, mutation도 `mut` callback으로 제한한다. Insert는 key/value ownership을
map으로 이동하고 replaced value가 있으면 `Option[V]`로 반환한다.
