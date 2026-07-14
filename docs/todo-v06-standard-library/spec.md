# Spec: v0.6-standard-library

상태: decision gate draft; user approval required

## 목표

- 실제 native CLI program에 필요한 arguments, environment, text, file I/O와 collection을
  ownership-safe standard package로 제공한다.
- Recoverable platform failure를 `Result`로 표현하고 compiler fatal runtime guard와 분리한다.
- v0.5의 move, call-scoped borrow, cleanup과 allocation contract를 모든 standard API에 적용한다.

## 추천 방향

- Standard library는 implicit global builtin을 늘리지 않고 `std.*` package로 제공한다.
- Existing `package`/`import`/visibility model을 재사용하되 `std` package source/runtime binding은
  compiler distribution이 소유한다.
- `func main()` signature는 유지하고 process state는 `std.os` 함수로 읽는다.
- `string`은 immutable UTF-8 text로 유지하며 byte length/index와 Unicode-aware operation 이름을
  구분한다.
- File and stream I/O는 `Result[T, Error]`를 반환하고 fatal runtime guard를 사용하지 않는다.
- Error propagation syntax는 API와 explicit `match` acceptance를 먼저 구현한 뒤 별도 승인한다.
- Key-value collection은 owned generic `Map[K, V]` standard type으로 시작하고 supported key type을
  명시적으로 제한한다.

## 구현 전 승인 항목

- `docs/todo-v06-standard-library/open-questions.md`의 Q1-Q7
- Standard package resolution과 runtime intrinsic boundary
- Text offset/Unicode semantics
- I/O error value와 propagation surface
- Map ownership, lookup return와 mutation semantics

## 완료 기준 초안

- Arguments/environment, text operation, file read/write와 map을 사용하는 CLI example이 native로
  compile/run된다.
- Every allocating API의 ownership transfer와 failure cleanup이 typed IR/backend regression으로
  고정된다.
- Recoverable error는 `Result`로 전달되고 fatal no-unwind guard와 혼동되지 않는다.
- Standard package generated C가 strict warning-clean과 ASan/UBSan gate를 통과한다.
- `SPEC.md`, API reference와 implementation이 같은 text/error/ownership model을 설명한다.
