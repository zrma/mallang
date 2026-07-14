# Open Questions: v0.4-generic-data-model

상태: awaiting user approval

v0.4 구현 전에 user-defined enum, generic surface, pattern 범위와 static
specialization 계약을 확정한다. Q1-Q7은 서로 연결된 하나의 language decision
gate이며 승인 전에는 parser나 semantic 구현을 시작하지 않는다.

## Q1. User-defined enum 문법

추천: `type Name[T] enum { ... }`과 payload가 없거나 정확히 하나인 variant

```mlg
type Maybe[T] enum {
    None
    Some(T)
}
```

- 기존 `type Name struct`와 같은 declaration family를 유지한다.
- 단일 payload는 nested generic type이나 struct를 담을 수 있다.
- 여러 값을 담는 variant는 별도 struct를 payload로 사용한다.
- Enum declaration의 visibility가 모든 variant에 적용되며 variant별 `pub`은 없다.
- Public enum의 payload type도 public API visibility 검사를 통과해야 한다.
- named payload와 tuple-like multiple payload는 v0.4에서 제외한다.

대안:

- `enum Maybe[T] { ... }`: 짧지만 기존 named type declaration 형태와 달라진다.
- `type Maybe[T] sum { ... }`: 의미는 분명하지만 새 keyword가 필요하다.

## Q2. Generic declaration과 사용 문법

추천: declaration과 use 모두 square bracket type argument를 사용하고 v0.4에서는
call-site type inference를 하지 않는다.

```mlg
type Box[T] struct {
    value T
}

func Identity[T](value T) T {
    return value
}

number := Identity[int](7)
identity := Identity[int]
```

- 기존 `Name[T]` type reference parser와 일치한다.
- 명시적 concrete type argument가 specialization key와 diagnostic을 안정시킨다.
- Generic function을 value로 사용할 때도 먼저 concrete function으로 만든다.

대안:

- `Identity(7)`에서 `T` 추론: 편리하지만 inference boundary와 ambiguity
  diagnostic을 함께 설계해야 한다.
- `Identity<int>(7)`: C++ 표기에 가깝지만 기존 type argument 표기와 다르다.

## Q3. Variant constructor 이름 해석

추천: user-defined variant는 항상 enum type으로 한정한다.

```mlg
some := Maybe[int].Some(7)
none := Maybe[int].None
ready := State.Ready
imported := model.Maybe[int].Some(7)
```

- package, type, variant 경계가 source에 남아 전역 variant 충돌을 피한다.
- 기존 selector와 square bracket 문법을 조합하며 별도 `::` token을 추가하지 않는다.
- Built-in `Some`, `None`, `Ok`, `Err` compatibility는 Q5에서 별도로 다룬다.

대안:

- 모든 variant를 unqualified value namespace에 넣기: 짧지만 package 안에서 쉽게
  충돌한다.
- `Maybe[int]::Some(7)`: 경계는 선명하지만 Go-like surface에 새 separator가 생긴다.

## Q4. v0.4 pattern 범위

추천: wildcard, zero/single payload binding, type-qualified nested variant pattern을
지원한다.

```mlg
match value {
    case Maybe.Some(Ok(item)) {
        print(item)
    }
    case Maybe.Some(Err(message)) {
        print(message)
    }
    case Maybe.None {
        print("none")
    }
}
```

- Pattern의 generic type argument는 scrutinee type에서 결정하며 쓰지 않는다.
- User-defined enum에도 exhaustiveness, duplicate arm, unreachable arm diagnostic을
  적용한다.
- Pattern guard, literal/range/or pattern은 v0.4에서 제외한다.

대안:

- v0.4에서 flat pattern만 지원: 구현은 작지만 generic nested ADT의 실용성이 낮다.
- guard와 literal pattern까지 함께 지원: pattern usefulness 알고리즘 범위가 크게
  늘어난다.

## Q5. Built-in `Option`과 `Result` 호환성

추천: compiler 내부에서는 predeclared generic enum metadata로 일반화하되 기존
source syntax는 유지한다.

- `Option[T]`, `Result[T, E]`, `Some`, `None`, `Ok`, `Err`는 계속 동작한다.
- 기존 program을 깨지 않도록 built-in variant의 unqualified spelling을 보존한다.
- `Option.Some`이나 `Result.Ok` alias는 v0.4에 추가하지 않는다.
- User-defined enum은 Q3의 qualified rule만 사용한다.

대안:

- Built-in도 즉시 `Option.Some`으로 변경: 모델은 단순하지만 v0 source를 깨뜨린다.
- Built-in ADT를 계속 별도 구현으로 유지: 단기 변경은 적지만 semantic/backend
  duplication이 남는다.

## Q6. Generic specialization과 ownership

추천: project-wide demand-driven monomorphization을 semantic 이후, typed IR 이전에
수행한다.

- Specialization key는 internal declaration symbol과 concrete type argument 목록이다.
- Generic body는 symbolic type parameter 상태로 한 번 검사한다.
- Constraint가 없는 type parameter는 generic body 안에서 보수적으로 non-Copy로
  취급한다.
- Unconstrained type parameter에는 모든 type에서 유효한 move/borrow와 generic
  signature operation만 허용하고 임의 arithmetic, equality, print는 거부한다.
- Concrete specialization은 기존 `is_copy`와 `needs_cleanup` 분류를 사용한다.
- 같은 key의 recursive request는 진행 중 specialization을 재사용한다.
- 계속 다른 key를 만드는 무한 specialization cycle은 source diagnostic으로
  거부한다.
- Runtime boxing, type descriptor, garbage collector, dynamic dispatch는 사용하지
  않는다.

대안:

- Type-erased boxed generic ABI: code size는 줄지만 runtime metadata와 uniform
  ownership ABI가 필요하다.
- 사용 지점마다 generic body를 처음부터 다시 검사: 단순해 보이지만 diagnostic과
  package semantics가 concrete use에 따라 달라진다.

## Q7. Generic receiver와 method 범위

추천: receiver type이 선언한 type parameter를 method scope에 도입한다.

```mlg
func (con box Box[T]) HasValue() bool {
    return true
}
```

- `T`는 `Box[T]` receiver에서 도입되고 concrete receiver specialization을 따른다.
- Method 자체가 별도의 type parameter 목록을 추가하는 기능은 v0.4에서 제외한다.
- Receiver에 나타나지 않는 type parameter는 거부한다.

대안:

- `func [T](con box Box[T]) ...`: type parameter 위치가 일반 function과 달라진다.
- Method별 독립 generic parameter 허용: selector resolution과 inference 범위가
  넓어진다.

## 추천안 승인 범위

Q1-Q7 추천안을 함께 승인하면 `spec.md`를 v0.4 구현 계약으로 확정하고 parser,
semantic, monomorphization, typed IR/C backend, native acceptance 순서로 구현한다.
승인되지 않은 동안 이 문서는 proposal이며 현재 지원 문법이 아니다.
