# Spec: v0.5-ownership-runtime

상태: released as v0.5.0 (2026-07-15; P138-P145 complete)

## 목표

- 사용자가 기본 move와 call-scoped `con`/`mut`만 기억하는 v1 memory model을
  완성한다.
- Pointer/reference wrapper 없이 recursive functional ADT를 ordinary value syntax로
  표현한다.
- Allocation, indirection, temporary lifetime과 drop을 compiler 내부 책임으로 둔다.
- Normal-flow cleanup과 fatal no-unwind runtime failure를 명확히 분리한다.

## 확정 결정

- `Box`, `Heap`, `Shared`, `Weak`, raw pointer와 lifetime syntax를 추가하지 않는다.
- User enum variant는 zero, one, multiple positional payload를 가질 수 있다.
- Recursive user enum은 source에서 ordinary move-only value이며 backend가 compiler-owned
  indirect storage로 낮춘다.
- Direct recursive struct와 base variant가 없는 비생산적 recursive ADT는 거부한다.
- General partial move, `replace` intrinsic과 struct destructuring은 추가하지 않는다.
- Full-expression temporary cleanup은 source syntax 없이 IR에서 구현한다.
- `con`/`mut` borrow는 call-scoped로 유지하고 by-reference range를 추가하지 않는다.
- Fatal runtime failure는 stderr diagnostic과 non-zero no-unwind termination을 사용한다.
- Static literal과 future heap-backed text는 별도 source type 없이 같은 immutable,
  move-only `string` value와 drop contract를 사용한다.

## Recursive Enum Surface

```mlg
type List[T] enum {
    Nil
    Cons(T, List[T])
}

func Prepend[T](head T, tail List[T]) List[T] {
    return List[T].Cons(head, tail)
}

func Length[T](list List[T]) int {
    return match list {
        case List.Nil { 0 }
        case List.Cons(_, tail) { 1 + Length[T](tail) }
    }
}
```

Rules:

- Variant declaration payload arity is part of the enum signature.
- Constructor type argument remains explicit under v0.4 rules.
- Constructor argument count and each payload type are checked at source span.
- Pattern payload count must exactly match the active variant arity.
- Each payload pattern may be a binding, wildcard or nested enum pattern.
- Match remains exhaustive and consuming for move-only scrutinees.
- Existing single payload syntax remains source-compatible.
- Named variant fields, tuple types, tuple expressions and struct patterns are not implied.

## Recursive Type Validity

The compiler builds a declaration graph after concrete generic specialization.

- An acyclic declaration keeps the existing inline representation.
- A cyclic component is valid only when it contains at least one user enum.
- Every valid cyclic component must have a constructor path that does not immediately require
  another value from the same component.
- Direct or mutual struct-only recursion is rejected.
- Built-in `Option`/`Result` alone do not create implicit recursive indirection.
- A recursive concrete specialization is move-only regardless of payload Copy properties.
- Expanding generic specialization cycles remain rejected under the v0.4 budget rule.

Examples:

```mlg
// valid: Nil is a base variant
type List[T] enum {
    Nil
    Cons(T, List[T])
}

// invalid: no finite constructor
type Loop enum {
    Again(Loop)
}

// invalid: no enum indirection boundary
type Node struct {
    next Node
}
```

## Ownership and Storage Contract

- Recursive enum constructor는 compiler-owned non-null storage를 만들 수 있다.
- Constructor arguments are evaluated left to right and then moved into active payload slots.
- A recursive enum handle has exactly one owner and cannot be copied or observed as an address.
- Assignment, argument passing and return transfer handle ownership under normal move rules.
- Consuming match transfers every active payload into an arm binding or wildcard cleanup slot.
- After payload transfer, the compiler releases the enum storage shell exactly once.
- Dropping an unmatched value drops active payloads before releasing its storage.
- Source semantics do not depend on the concrete C pointer/tag/union layout.
- Allocation size overflow and allocation failure become Mallang fatal runtime errors.

## Aggregate and Field Rules

- User struct fields remain fully initialized for the lifetime of the struct.
- Plain move-out from a non-slice struct field remains rejected.
- Field assignment evaluates the replacement first, drops the old field, then stores the new
  ownership.
- Existing owned slice field take leaves an empty slice and remains the only field-take
  compatibility exception.
- Enum match consumes the whole enum, so binding multiple payloads does not create a partially
  initialized parent value.

## String Runtime Contract

- `string` is one immutable, move-only source type regardless of runtime storage.
- Static literals and owned buffers carry the same byte-sequence and length value semantics.
- Storage kind, data address and allocation strategy are compiler/runtime details and are not
  source-observable.
- Equality compares length and bytes; `print` writes the same bytes. Both operations borrow for
  the operation and do not move the value.
- Move transfers storage responsibility. Drop does not free static storage and frees owned
  storage exactly once on normal control flow.
- Parameter, return, local, struct field, enum payload and closure capture use the common cleanup
  path.
- Overwrite evaluates the replacement first and the target place once, then drops the old value
  before storing the replacement.
- Mutable borrowed parameters and mutable closure captures keep the replacement owned by their
  external owner after the call.
- Malformed storage/data, allocation-size overflow and allocation failure are fatal no-unwind
  runtime errors.
- String-producing standard-library operations are deferred to v0.6; v0.5 provides their runtime
  ownership representation and internal allocation contract.

## Temporary and Cleanup Rules

Every owned value on normal control flow is moved or dropped exactly once.

- Full-expression temporaries are explicit in typed IR, not source syntax.
- Call argument temporaries live through the call and drop after return.
- Condition/index/`len` temporaries drop after their final read.
- Branch-local values drop when the selected branch exits.
- Loop source temporaries live through the loop and drop on normal exit, `break` or enclosing
  function return.
- Return expressions are evaluated into caller-owned return temporaries before callee local
  cleanup.
- Overwrite evaluates the right-hand side before dropping the old destination.
- Cleanup-valued expression statements drop their result at full-expression end.
- A control-flow merge that cannot prove one ownership state must be normalized explicitly or
  rejected; the backend does not guess with implicit aliasing.

## Borrow and Range Rules

- `con expr` and `mut expr` remain direct call argument modes.
- Using either marker in a local initializer, return value or other expression position is a
  reserved diagnostic rather than a first-class reference expression.
- A borrow ends when the callee returns.
- Borrowed non-Copy values cannot be moved, returned, stored or captured.
- First-class references, lifetime annotation and borrowed return values are not part of v1.
- Range value binding remains Copy-only.
- Non-Copy range traversal uses index-only iteration and call-scoped indexed borrow.
- Range element mutation uses indexed assignment or `mut values[i]` call access.
- `for i, con value := range values` and `for i, mut value := range values` are reserved
  diagnostics with the index-only alternatives named in the message.
- Compiler-owned loop temporaries do not create user-visible borrowed values.

## Runtime Failure Contract

- Fatal guards print a stable `mallang runtime error: ...` diagnostic to stderr.
- Fatal failure exits non-zero and does not unwind Mallang stack frames.
- Normal program exit must release all compiler-owned allocations.
- User-visible panic/recover, exception and catch are not added.
- Recoverable I/O and environment failures belong to v0.6 `Result` APIs.

## Allocation Accounting Contract

- Slice buffer, closure environment, recursive enum node와 owned string buffer는 generated C의
  공통 compiler-owned allocation helper를 사용한다.
- 새 storage lifetime을 만드는 successful allocation은 live count를 1 증가시킨다. Existing
  allocation의 realloc growth는 count를 유지하고 null buffer의 first growth는 1 증가시킨다.
- Non-null deallocation은 live count를 정확히 1 감소시키며 null deallocation은 no-op이다.
- Live count가 0인데 non-null storage를 해제하려는 경로는 internal fatal accounting error다.
- Internal test build는 source/API 변경 없이 N번째 allocation attempt를 deterministic하게
  실패시킬 수 있다. Diagnostic은 `slice allocation failed`처럼 allocation site 의미를 유지한다.
- Normal `main` return은 compiler-owned live allocation count 0을 만족해야 한다.
- Fatal no-unwind failure는 pending value cleanup이나 zero live count를 보장하지 않는다.

## 구현 순서

1. Q1-Q7 추천안을 승인하고 이 문서를 확정한다. (완료)
2. Enum declaration/constructor/pattern을 positional payload list로 일반화한다. (완료: P138)
3. Concrete recursive declaration graph, base-case validation과 diagnostics를 추가한다. (완료: P139)
4. Recursive multi-payload enum typed IR과 ownership transfer를 추가한다. (완료: P140)
5. Compiler-owned recursive enum C layout, constructor, match와 drop을 구현한다. (완료: P141)
6. Full-expression temporary와 loop source cleanup normalization을 완성한다. (완료: P142)
7. Static/owned string runtime representation과 drop contract를 통합한다. (완료: P143)
8. Borrow/range exclusion regression과 normative memory spec을 동기화한다. (완료: P144)
9. Allocation accounting, failure injection, strict C와 sanitizer acceptance를 연결한다.
   (완료: P145)

## 제외

- `Box`, `Heap`, `Shared`, `Weak` source type
- Raw pointer, address-of, dereference와 nullable handle
- Shared ownership, reference counting와 cycle collector
- First-class reference와 user-visible lifetime
- By-reference/mutable range binding
- General partial move, `replace` intrinsic와 struct destructuring
- Named enum payload와 standalone tuple type
- Direct/mutual struct-only recursive value type
- User-defined destructor/finalizer
- Panic/recover, exception와 stack unwinding

## 완료 기준

- Generic recursive enum이 multiple positional payload로 native compile/run된다.
- Recursive constructor/match/drop이 non-Copy nested payload를 정확히 한 번 정리한다.
- Invalid payload arity/type, non-productive recursion과 struct-only recursion을 source
  diagnostic으로 거부한다.
- Full-expression temporary가 call, condition, index, range와 discarded expression에서
  leak이나 double-drop 없이 동작한다.
- Known use-after-move, borrowed escape, borrow overlap과 cleanup merge failure가 regression으로
  고정된다.
- Return, branch, loop, early exit와 overwrite cleanup이 typed IR regression으로 고정된다.
- Cleanup-heavy generated C가 strict warning-clean, ASan/UBSan과 normal-exit allocation
  accounting을 통과한다.
- `SPEC.md`, implementation과 runtime failure behavior가 같은 memory model을 설명한다.
