# Open Questions: v0.5-ownership-runtime

상태: closed

2026-07-14에 user-visible ownership surface를 최소화한 Q1-Q7 추천안을 승인했다.

v0.5는 memory implementation concept를 새 syntax로 노출하지 않는다. 사용자가
기억할 ownership model은 기본 move와 call-scoped `con`/`mut`뿐이다. Recursive
ADT의 allocation/indirection, temporary lifetime과 drop은 compiler가 책임진다.

## 현재 Gap Inventory

- Copy/move/drop 분류가 여러 boolean helper와 backend branch에 나뉘어 있다.
- Slice와 closure는 compiler-owned heap allocation을 사용하지만 recursive owned
  user data는 아직 거부된다.
- User enum은 payload가 없거나 하나인 variant만 지원해 functional recursive ADT를
  struct partial move 없이 분해하기 어렵다.
- Named cleanup root의 주요 branch/loop/return path는 지원하지만 arbitrary owned
  temporary의 full-expression lifetime은 일반화되지 않았다.
- Non-slice field move는 거부하고 slice field take만 empty replacement로 허용한다.
- Runtime guard는 stderr를 출력하고 process를 종료하며 stack unwinding은 하지 않는다.

## Q1. User-visible ownership surface

추천: v1 후보 surface에도 새 pointer/reference ownership type을 추가하지 않는다.

```text
기본 전달 = ownership move
con 전달 = call 동안 read borrow
mut 전달 = call 동안 exclusive mutable borrow
```

- `Box`, `Heap`, `Shared`, `Weak`, address-of, dereference와 lifetime syntax를 추가하지
  않는다.
- Shared ownership, reference counting과 cycle collector를 추가하지 않는다.
- Compiler-owned allocation은 source value의 단일 소유 semantics를 바꾸지 않는다.
- Allocation identity와 address는 Mallang program에서 관찰할 수 없다.

대안:

- Explicit `Box[T]`: allocation cost는 보이지만 recursive value를 작성할 때
  `box`/`unbox`와 projection rule을 계속 의식해야 한다.
- `Shared[T]`/`Weak[T]`: graph modeling은 쉬워지지만 reference count, cycle과 weak
  upgrade semantics가 language core로 들어온다.

## Q2. Recursive ADT source surface

추천: user enum variant에 positional payload를 여러 개 허용하고 recursive enum을
ordinary value syntax로 작성한다.

```mlg
type List[T] enum {
    Nil
    Cons(T, List[T])
}

func Length[T](list List[T]) int {
    return match list {
        case List.Nil { 0 }
        case List.Cons(head, tail) { 1 + Length[T](tail) }
    }
}
```

- Zero, one, multiple positional payload를 지원한다.
- Constructor는 `List[int].Cons(1, tail)`, pattern은
  `List.Cons(head, tail)` 형태다.
- Pattern은 active payload 전체를 분해한다. Payload 일부만 생략하려면 해당 위치에
  `_`를 쓴다.
- Named payload와 struct pattern은 v0.5에 추가하지 않는다.
- Existing zero/single payload enum source는 그대로 동작한다.

Multiple positional payload는 별도 tuple type을 추가하지 않고 recursive functional
data를 consume-match로 안전하게 분해하기 위한 최소 surface다.

## Q3. Compiler-owned recursive representation

추천: recursive user enum을 compiler-owned indirect representation으로 낮춘다.

- Recursive declaration graph의 cyclic strongly connected component를 계산한다.
- Cycle에 참여하는 user enum value는 backend에서 non-null owned handle로 표현한다.
- Struct는 inline value로 유지한다. Enum을 통하지 않는 direct/mutual recursive
  struct는 계속 거부한다.
- Recursive SCC에는 recursive payload를 사용하지 않는 base variant가 하나 이상
  있어야 한다. `type Loop enum { Again(Loop) }` 같은 비생산적 type은 거부한다.
- Constructor는 owned storage를 만들고 payload ownership을 그 storage로 move한다.
- Consuming match는 storage를 열어 payload ownership을 arm binding으로 move하고
  storage shell을 정리한다.
- Drop은 active payload를 먼저 drop하고 storage를 정확히 한 번 해제한다.
- Recursive constructor가 allocation할 수 있다는 사실은 language/runtime contract에
  문서화하지만 allocation handle을 source에 노출하지 않는다.
- Backend가 storage layout을 최적화해도 source ownership semantics는 바뀌지 않는다.

대안:

- 모든 recursive field를 source-level wrapper로 표시: layout은 명시적이지만 syntax와
  ownership operation이 늘어난다.
- 모든 user enum을 항상 heap handle로 표현: backend는 단순하지만 non-recursive
  small enum에도 불필요한 allocation이 생긴다.

## Q4. Field move와 aggregate initialization

추천: general uninitialized partial move를 열지 않는다.

- Consuming enum match가 active payload 전체를 binding 또는 wildcard로 처리한다.
- User struct의 non-slice field를 plain field access로 move하는 것은 계속 거부한다.
- Mutable field assignment는 old value를 안전하게 drop한 뒤 fully initialized value로
  교체한다.
- Existing owned slice field take는 empty slice replacement를 사용하는 compatibility
  exception으로 유지한다.
- `replace` intrinsic, partially initialized parent, projection별 runtime drop flag와
  struct destructuring은 v0.5에서 제외한다.

Q2의 positional enum payload가 recursive functional data의 주요 decomposition use
case를 처리하므로 general partial move를 함께 열 필요가 없다.

## Q5. Owned temporary lifetime

추천: 모든 owned expression 결과를 compiler IR에서 full-expression temporary로
정규화하고 normal control flow에서 결정적으로 drop한다.

- Call argument, condition, index, `len`, nested constructor와 discarded expression의
  cleanup temporary를 마지막 사용 뒤 drop한다.
- `len([]int{1, 2})`, temporary indexing과 call-scoped temporary borrow를 허용한다.
- `for ... range makeValues()` source는 compiler-owned loop temporary에 보관하고
  normal loop exit, `break`와 enclosing function return path에 맞춰 drop한다.
- Return expression은 return temporary로 먼저 평가하고 callee local cleanup 뒤
  caller에게 ownership을 전달한다.
- Assignment는 right-hand side 평가를 완료한 뒤 old destination을 drop하고 새
  ownership을 저장한다.
- Source에는 temporary, drop 또는 lifetime syntax를 추가하지 않는다.

## Q6. First-class reference와 range

추천: `con`/`mut`는 call-scoped로 유지하고 first-class reference와 by-reference range
binding을 v1 후보 surface에서 제외한다.

- Borrowed value는 local, field, return value 또는 closure capture로 저장할 수 없다.
- `for i, value := range values`는 Copy value iteration으로 유지한다.
- Non-Copy iteration은 `for i := range values`와 indexed borrow를 사용한다.
- Element mutation은 `values[i] = value` 또는 call-scoped `mut values[i]`를 사용한다.
- `for i, con value := range values`와 `for i, mut value := range values`는 추가하지
  않는다.
- Q5의 range source temporary는 compiler가 source value를 loop scope에 소유하는
  것이며 user-visible borrowed binding이 아니다.

## Q7. Drop, runtime failure와 acceptance

추천: compiler-owned deterministic normal-flow drop과 no-unwind fatal failure를
분리한다.

- `int`, `bool`, `unit`은 `Copy`다.
- Built-in `Option`/`Result`의 기존 conditional Copy rule을 유지한다.
- `string`, fixed array, owned slice, user struct/enum과 function value는 move-only다.
- Recursive enum은 항상 move-only다.
- Future heap-backed string과 current static literal은 같은 immutable `string` type과
  drop contract를 사용한다.
- User-defined destructor/finalizer는 추가하지 않는다.
- Bounds, overflow, invalid runtime tag와 allocation failure는 diagnostic을 stderr에
  쓰고 non-zero로 종료한다.
- Fatal termination은 Mallang stack을 unwind하지 않는다. Recoverable failure는 v0.6
  standard library의 `Result` API로 전달한다.
- Static rejection, typed IR drop regression, strict generated C, ASan/UBSan과
  normal-exit allocation accounting을 v0.5 acceptance에 포함한다.

## 승인 범위

Q1-Q7을 `spec.md`의 v0.5 구현 계약으로 확정한다. 새 ownership syntax는 추가하지
않으며 recursive positional ADT와 compiler cleanup 구현을 순서대로 진행한다.
