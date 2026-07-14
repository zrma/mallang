# Spec: v0.4-generic-data-model

상태: approved, implementation in progress

## 목표

- Built-in `Option`/`Result`에 한정된 sum type을 user-defined generic enum으로
  일반화한다.
- Generic type과 function을 정적 specialization하여 기존 native value model과
  ownership/cleanup 규칙을 유지한다.
- User-defined enum의 nested destructuring과 exhaustive match를 source diagnostic,
  typed IR, C backend까지 end-to-end로 연결한다.

## 제안 surface

이 절은 승인된 `open-questions.md` Q1-Q7의 구현 계약이다.

### Generic type과 function

```mlg
type Pair[A, B] struct {
    first A
    second B
}

func Identity[T](value T) T {
    return value
}

pair := Pair[int, string]{first: 7, second: "mallang"}
first := pair.first
identity := Identity[int]
same := identity(first)
```

- Type parameter는 declaration name 바로 뒤의 `[T, U]`에 선언한다.
- Generic type/function use에는 concrete type argument를 명시한다.
- Type argument 개수, unknown type parameter, non-concrete specialization을 source
  diagnostic으로 거부한다.
- Generic function value는 `Identity[int]`처럼 concrete specialization만
  first-class value가 될 수 있다.

### User-defined enum

```mlg
type Maybe[T] enum {
    None
    Some(T)
}

type Lookup[T] enum {
    Missing
    Found(T)
}

value := Maybe[int].Some(7)
```

- Variant는 payload가 없거나 정확히 하나다.
- User-defined variant constructor는 enum type으로 한정한다.
- Generic enum type argument는 constructor expression에 명시한다.
- Enum declaration visibility가 모든 variant에 적용되고 variant별 visibility는 없다.
- Public enum payload type은 기존 public API visibility 검사를 통과해야 한다.
- Concrete enum은 tag와 payload union을 가진 typed value로 lowering한다.

### Pattern과 match

```mlg
func Unwrap(value Maybe[Result[int, string]]) int {
    return match value {
        case Maybe.Some(Ok(item)) { item }
        case Maybe.Some(Err(message)) { 0 }
        case Maybe.None { 0 }
    }
}
```

- Pattern에서는 scrutinee가 concrete enum type을 제공하므로 type argument를
  반복하지 않는다.
- Wildcard `_`, zero-payload variant, single-payload binding과 nested variant를
  지원한다.
- Match는 concrete enum variant 전체를 다루거나 wildcard arm을 가져야 한다.
- Duplicate variant, wildcard 뒤 arm, payload arity/type mismatch를 거부한다.

### Generic receiver

```mlg
type Box[T] struct {
    value T
}

func (con box Box[T]) IsSet() bool {
    return true
}
```

- Receiver에 나타난 generic type parameter가 method scope에 들어온다.
- Concrete receiver method lookup은 receiver type argument를 method body와
  signature에 대입한다.
- Independently generic method는 지원하지 않는다.

## Built-in ADT migration

- `Option[T]`와 `Result[T, E]`는 predeclared generic enum metadata로 표현한다.
- 기존 `Some`, `None`, `Ok`, `Err` constructor와 pattern spelling은 호환성을 위해
  유지한다.
- User-defined enum과 built-in ADT는 같은 type checking, exhaustiveness, layout,
  cleanup 경로를 사용한다.
- Built-in 전용 semantic/backend 분기는 일반화 뒤 제거하거나 compatibility
  spelling 해석에만 남긴다.

## Static specialization 계약

1. Package linking 뒤 generic declaration graph와 concrete demand를 수집한다.
2. Internal declaration symbol과 concrete type argument 목록으로 specialization
   key를 만든다.
3. Generic body는 symbolic parameter로 한 번 검사하고 unconstrained parameter를
   non-Copy로 취급한다.
4. Demand-driven worklist가 concrete type/function/method specialization을 생성한다.
5. Concrete ownership와 cleanup classification을 계산한 뒤 typed IR로 lowering한다.
6. Backend에는 generic parameter가 남지 않은 concrete declaration만 전달한다.

같은 specialization key는 project 전체에서 재사용한다. Direct recursion은 진행 중
key를 참조할 수 있지만 type argument가 계속 커지는 specialization chain은 유한성
검사로 거부한다. 별도 compilation artifact나 generic ABI는 v0.4 범위가 아니다.
Unconstrained type parameter에는 move/borrow와 generic signature로 증명되는
operation만 허용하며 arbitrary arithmetic, equality, print는 거부한다.

## Ownership과 cleanup

- Generic value의 기본 전달은 concrete type이 non-Copy이면 move다.
- `con`/`mut` parameter와 receiver는 concrete specialization에서도 기존 call-scoped
  borrow 규칙을 유지한다.
- Generic body는 unconstrained `T`를 복사할 수 있다고 가정하지 않는다.
- Concrete enum/struct의 payload와 field cleanup은 specialization된
  `needs_cleanup` 결과를 따른다.
- Constructor failure, overwrite, branch, return, match payload move에서 non-Copy
  value가 정확히 한 번 정리되어야 한다.
- Match arm이 payload를 소유 binding으로 꺼내면 scrutinee의 해당 payload는 moved
  상태로 처리한다.

## 구현 순서

1. Q1-Q7 language decision을 승인받고 이 문서를 확정한다.
2. Generic parameter와 enum declaration, specialized constructor/pattern AST를
   추가한다. (완료)
3. Package symbol/visibility와 generic type/function/receiver resolution을 추가한다.
   (완료: enum type metadata와 public payload visibility 포함)
4. Symbolic generic checker와 project-wide concrete specialization worklist를 추가한다.
   (완료: generic enum과 source diagnostic 복원 포함)
5. User-defined enum constructor, exhaustiveness와 nested pattern diagnostics를 일반화한다.
   (진행 중: constructor specialization/semantic 완료, pattern/exhaustiveness 미완료)
6. Specialized typed IR, concrete layout, constructor/match C lowering을 추가한다.
   (진행 중: struct/function은 concrete AST를 기존 typed IR/C backend로 전달,
   enum layout과 constructor/match lowering 미완료)
7. Built-in `Option`/`Result`를 공통 generic enum 경로로 이전한다.
8. Cross-package positive smoke, invalid fixture, strict C와 sanitizer gate를 추가한다.
   (진행 중: generic struct/function/receiver positive native gate 완료, enum과 invalid
   fixture 미완료)

## 제외

- call-site type inference
- interface/trait constraint와 dynamic dispatch
- specialization syntax
- higher-kinded type, associated type, const/default type argument
- multiple/named variant payload
- pattern guard, literal/range/or pattern
- independently generic method
- owned recursive by-value/heap data model
- separate compilation용 generic binary artifact

## 완료 기준

- User-defined generic struct와 enum을 각각 둘 이상의 concrete type으로 native
  compile/run한다.
- Generic function, function value와 receiver method가 concrete specialization으로
  동작한다.
- User-defined enum의 exhaustive nested match가 expression과 statement에서 동작한다.
- Built-in `Option`/`Result` 기존 source가 호환성을 유지하면서 공통 경로를 쓴다.
- Invalid type argument, constructor, pattern, non-Copy misuse와 infinite
  specialization을 source diagnostic으로 거부한다.
- Non-Copy generic payload의 move/drop이 strict generated C와 sanitizer sweep을
  통과한다.
- Multi-package generic API가 visibility와 deterministic specialization 규칙을
  지킨다.
