# Open Questions: v0.5-ownership-runtime

상태: proposed, approval required

v0.5 구현 전에 v1 memory model, owned heap indirection, field extraction,
temporary cleanup과 borrow/runtime failure 경계를 확정한다. Q1-Q7은 서로 연결된
하나의 language decision gate이며 승인 전에는 lexer/parser, semantic 또는 backend
구현을 시작하지 않는다.

## 현재 Gap Inventory

- `Type::is_copy`와 `Type::needs_cleanup`은 분리된 boolean 규칙이며 v1 전체 type의
  storage/drop 계약을 하나의 표로 표현하지 않는다.
- `string`은 move-only지만 현재 literal pointer만 사용해 cleanup 대상이 아니다.
  v0.6의 file I/O와 text operation은 heap-backed string을 필요로 한다.
- Slice와 closure는 compiler-owned heap allocation을 사용하지만 recursive owned
  value를 표현하는 source type은 없다.
- Cleanup insertion은 named root와 주요 branch/loop/return 경로를 다루지만 owned
  temporary의 full-expression lifetime은 아직 일반화되지 않았다.
- Non-slice field move는 거부하며 slice field take만 empty slice replacement로
  안전하게 허용한다.
- Borrow는 call-scoped이고 range body 전체에 걸친 borrowed binding이나 first-class
  reference는 없다.
- Runtime guard는 stderr를 출력하고 process를 종료하며 stack unwinding은 하지 않는다.

## Q1. v1 value와 drop 분류

추천: 현재 Copy 경계를 보존하고 모든 concrete type을 하나의 compiler-owned
`ValueKind`/`DropKind` 분류로 계산한다.

- `int`, `bool`, `unit`은 `Copy`다.
- `Option[T]`는 `T`, `Result[T, E]`는 두 payload가 모두 `Copy`일 때만 `Copy`다.
- `string`, fixed array, owned slice, user struct/enum, function value/closure와 Q2의
  `Heap[T]`는 move-only다.
- User-defined aggregate의 implicit structural Copy는 v0.5에 추가하지 않는다.
  추후 필요하면 명시적 derive/constraint와 함께 별도 decision gate로 연다.
- `string`은 static literal과 future heap-backed text를 같은 immutable owned type으로
  표현한다. Drop은 storage kind를 보고 static storage에는 no-op, owned storage에는
  deallocation을 수행한다.
- User-defined destructor/finalizer는 v0.5에 추가하지 않는다. 모든 drop은
  compiler/runtime-owned이고 panic하지 않는다.

대안:

- 모든 structurally Copy aggregate를 자동 Copy로 만들기: 편리하지만 large array와
  user aggregate의 암묵적 복사 비용이 커지고 현재 move 규칙이 바뀐다.
- `Option`/`Result`도 항상 move-only로 만들기: user enum과 단순해지지만 기존 source의
  Copy 재사용을 깨뜨린다.

## Q2. Owned recursive heap value

추천: predeclared move-only `Heap[T]`와 `heap(value)`, `unheap(value)` intrinsic을
추가한다.

```mlg
type List[T] enum {
    Nil
    Cons(Node[T])
}

type Node[T] struct {
    head T
    tail Heap[List[T]]
}

tail := heap(List[int].Nil)
list := List[int].Cons(Node[int]{head: 1, tail: tail})
```

- `heap(value)`는 `value`를 move해 non-null compiler-owned allocation에 저장한다.
- `unheap(boxed)`는 `Heap[T]` 전체를 consume하고 allocation shell을 해제한 뒤 `T`를
  반환한다.
- `boxed.value`는 compiler-known place projection이다. Copy read, nested field/index
  access와 `con`/`mut` call argument로 사용할 수 있지만 payload를 직접 move하지는
  않는다. Owned extraction은 `unheap`만 사용한다.
- `Heap[T]` drop은 active `T`를 drop한 뒤 allocation을 정확히 한 번 해제한다.
- Recursive type 검사에서 `Heap[T]`는 owned indirection boundary다.
- Null, address-of, dereference, pointer arithmetic과 nullable heap handle은 없다.
- Allocation failure는 Q6의 fatal runtime failure다.

`Box[T]`를 predeclared 이름으로 사용하지 않는다. v0.4에서 ordinary user generic
type `Box[T]`가 이미 유효하고 checked-in source에도 있으므로 이를 예약하면 기존
Mallang source를 불필요하게 깨뜨린다.

대안:

- `Box[T]`: Rust 사용자에게 익숙하지만 기존 Mallang user type과 충돌한다.
- implicit heap allocation for recursive fields: source는 짧지만 allocation과 failure
  cost가 type에서 보이지 않는다.
- reference-counted shared pointer: aliasing, cycles와 atomicity 정책이 필요해 v0.5
  단일-owner 모델보다 훨씬 넓다.

## Q3. Field extraction과 partial move

추천: v1에서도 live aggregate는 항상 완전히 초기화된 상태를 유지하고 general
uninitialized partial move를 열지 않는다. 대신 compiler intrinsic
`replace(mut place, replacement) -> T`를 추가한다.

```mlg
oldProfile := replace(mut user.profile, Profile{name: "new"})
oldNode := replace(mut heapNode.value, Node{...})
```

- `place`는 mutable local, local-rooted field/index path 또는 `Heap[T].value`다.
- Place의 base와 index를 정확히 한 번 평가하고, replacement를 먼저 안전한 temporary로
  만든 뒤 old value를 결과로 move하고 replacement를 저장한다.
- Old value는 반환되므로 overwrite drop을 하지 않는다. 반환값은 일반 ownership
  규칙에 따라 consume하거나 scope에서 drop한다.
- Existing owned slice field take는 empty slice로 replace하는 compatibility sugar로
  유지한다.
- Plain non-slice field access로 move-out하는 규칙, uninitialized field state,
  path-dependent runtime drop flag와 partially moved parent 사용은 계속 제외한다.

대안:

- Rust-style arbitrary partial move: 표현력은 높지만 projection별 initialization state,
  branch merge와 conditional drop flag가 필요하다.
- 모든 field move를 금지: 구현은 단순하지만 owned aggregate를 안전하게 변환하기
  어렵다.

## Q4. Owned temporary lifetime

추천: 모든 owned expression 결과를 full-expression temporary로 정규화하고 정상
control flow에서 결정적으로 drop한다.

- Call argument, condition, index, `len`, nested constructor와 expression statement에서
  만들어진 cleanup value는 마지막 사용 뒤 drop한다.
- `len([]int{1, 2})`, temporary slice indexing과 call-scoped temporary borrow를
  허용한다.
- `for ... range makeValues()`의 owned source는 loop 전 temporary에 보관하고 loop
  종료, `break` 또는 function exit의 올바른 경로에서 drop한다.
- Cleanup value expression statement는 full-expression 끝에서 drop하므로 허용한다.
- Return value는 caller-owned temporary로 먼저 평가하고 callee local cleanup 뒤
  반환한다.
- Assignment/replace는 right-hand side를 먼저 평가한 뒤 old destination cleanup 또는
  ownership transfer를 수행한다.
- Branch나 loop merge에서 cleanup state를 증명하지 못하면 backend flag로 추측하지
  않고 source diagnostic으로 거부하거나 IR normalization을 추가한다.

대안:

- 현재 local-rooted 제한 유지: 구현은 작지만 v1 standard library와 value-oriented
  composition을 과도하게 제한한다.
- tracing GC 추가: temporary cleanup은 쉬워지지만 Mallang의 single-owner 목표와
  native deterministic cleanup을 바꾼다.

## Q5. First-class reference와 range

추천: v1에서도 `con`/`mut` borrow를 call-scoped로 유지하고 first-class reference,
user-visible lifetime, by-reference range binding을 추가하지 않는다.

- Borrowed value를 local, field, return value 또는 closure capture로 저장할 수 없다.
- `for i, value := range values`는 Copy value iteration, `for i := range values`는
  index-only iteration으로 유지한다.
- Element mutation은 `values[i] = replacement`, `replace(mut values[i], replacement)`
  또는 call-scoped `mut values[i]`를 사용한다.
- `for i, con value := range values`와 `for i, mut value := range values`는 v1 후보
  surface에서 제외한다.
- Q4의 range temporary는 compiler가 source value를 loop scope에 소유하는 것이며
  user-visible borrowed binding을 만들지 않는다.

현재 v1 CLI/collection use case는 indexed read/mutation과 owned iteration으로 처리할 수
있어 statement-spanning borrow의 복잡성을 정당화하지 않는다.

대안:

- Rust-style reference/lifetime: 표현력은 가장 높지만 language surface와 checker
  복잡도가 크게 증가한다.
- Range body scoped borrow binding만 특례로 추가: reference가 아니라고 이름 붙여도
  escape, capture, overlap과 mutation 규칙은 동일하게 필요하다.

## Q6. Runtime failure와 unwinding

추천: v0.5 fatal runtime failure는 no-unwind process termination으로 고정한다.

- Bounds, arithmetic overflow, invalid runtime tag와 allocation failure는
  `mallang runtime error: ...`를 stderr에 쓰고 non-zero로 종료한다.
- Fatal termination에서는 Mallang local drop이나 stack unwinding을 보장하지 않는다.
  Process 종료 시 OS가 resource를 회수하며 runtime은 Mallang code로 복귀하지 않는다.
- User-visible `panic`, `recover`, exception과 catch는 v0.5에 추가하지 않는다.
- Recoverable/expected failure는 v0.6 standard library에서 `Result`로 전달한다.
- Runtime allocation helper는 overflow를 먼저 검사하고 null allocation을 Mallang
  fatal error로 바꾸며 unchecked C allocation 결과를 노출하지 않는다.

대안:

- Full unwinding: destructor ordering, unwind ABI와 foreign frame 정책이 필요하다.
- Abort without diagnostic: 구현은 짧지만 current CLI error contract보다 약하다.

## Q7. Memory-safety acceptance

추천: static rejection, typed IR cleanup proof, generated C 검증과 native allocation
accounting을 함께 v0.5 완료 gate로 둔다.

- Type별 Copy/move/drop matrix를 spec과 compiler의 single source of truth에 맞춘다.
- Use-after-move, double consume, borrowed escape, overlapping mut borrow, partial move와
  cleanup state merge 실패를 source regression으로 고정한다.
- Return, branch, loop, `break`/`continue`, overwrite, replace, temporary와 recursive
  `Heap`의 exactly-once drop을 IR/backend regression으로 고정한다.
- Cleanup-heavy generated C 전체를 strict warning-clean과 ASan/UBSan으로 실행한다.
- Slice, string, closure와 `Heap` normal-exit smoke는 test allocator accounting으로
  live allocation이 0이 되는지 검증한다.
- Allocation failure injection은 Mallang runtime diagnostic과 non-zero exit를 검증한다.

## 추천안 승인 범위

Q1-Q7 추천안을 `spec.md`의 v0.5 구현 계약으로 확정한다. 추가 language surface나
기존 source를 깨는 변경이 필요할 때만 새 decision gate를 연다.
