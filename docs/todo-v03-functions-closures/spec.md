# Spec: v0.3-functions-closures

상태: approved, implementation in progress

## 목표

- Mallang의 함수형 value style을 named function call에서 first-class function과
  safe closure로 확장한다.
- 함수값의 move/borrow와 closure environment cleanup을 기존 ownership 모델에
  포함한다.
- escaping closure가 source-level pointer나 lifetime 없이 dangling reference를
  만들지 못하게 한다.

## 현재 구현 기반

- Named function은 value position에서 signature를 가진 fresh move-only function value로
  해석된다.
- `con`과 `mut`는 call-scoped read/exclusive borrow이고 non-copy value는 기본 move다.
- Struct, owned slice, built-in ADT cleanup이 typed IR와 C backend에 연결돼 있다.
- Package linker가 qualified direct call과 public declaration visibility를 처리한다.
- Function type과 closure literal AST/parser, function value/indirect call semantic이
  native callable ABI까지 연결돼 있다.
- Named function thunk, owned capture 분석, typed closure environment와 exactly-once
  cleanup이 구현돼 있다.
- Plain/mutable closure는 Copy, owned slice, nested callable capture 상태를 native로
  유지하며 generated C warning/sanitizer gate를 통과한다.
- Package-qualified function value와 project-level closure acceptance는 남아 있다.

## 추천 surface

### Function type과 value

- Go 형태의 `func(<parameter-types>) <return-type>`을 사용한다.
- Function type의 return type은 필수이며 반환값이 없으면 `unit`을 쓴다.
- Parameter mode는 function type의 일부다. 이름은 type에서 생략한다.
- Named function은 같은 signature의 function value로 사용할 수 있다.
- Package-qualified named function도 value position에서 사용할 수 있다.

```mlg
func Apply(value int, transform func(int) int) int {
    return transform(value)
}

func Double(value int) int {
    return value * 2
}

func main() {
    transform := Double
    print(Apply(21, transform))
}
```

`func(con string) int`와 `func(mut Counter) int`처럼 parameter mode가 다르면
서로 다른 function type이다.

### Closure literal

- Go 형태의 anonymous `func` literal을 사용한다.
- Free local binding은 compiler가 찾아 closure 생성 시 value로 capture한다.
- `Copy` capture는 복사되고 non-copy capture는 closure environment로 이동한다.
- Plain closure의 capture는 closure body에서 immutable이다.

```mlg
func MakeAdder(offset int) func(int) int {
    return func(value int) int {
        return value + offset
    }
}
```

### Mutable closure

- Capture를 변경하는 closure는 `func mut(...)` literal과 type으로 구분한다.
- Mutable closure는 `mut` binding에 저장했거나 `mut` parameter로 빌렸을 때만
  호출할 수 있다.
- 변경할 capture의 원본 binding도 closure 생성 시점에 `mut`여야 한다.
- Mutable closure가 소유한 capture의 변경은 이동 전 원본 binding에 반영되지 않는다.

```mlg
func MakeCounter() func mut(int) int {
    mut count := 0
    return func mut(delta int) int {
        count = count + delta
        return count
    }
}

func main() {
    mut counter := MakeCounter()
    print(counter(1))
    print(counter(2))
}
```

`func mut`은 call effect를 type에 보존한다. `func(...)` value는 `con` access로
반복 호출할 수 있고, `func mut(...)` value 호출은 exclusive access가 필요하다.

### Capture와 escape

- v0.3 capture는 owned-by-value만 지원한다.
- Closure는 local scope, parameter, return value, struct field에서 사용할 수 있다.
- Borrowed non-copy parameter나 active `con`/`mut` borrow를 capture하려 하면
  diagnostic으로 거부한다.
- Explicit capture list와 borrowed capture는 도입하지 않는다.
- Closure initializer 안에서 자기 binding을 capture하는 recursive closure는
  v0.3에서 거부한다.

Owned capture만 허용하면 escaping closure의 environment가 stack local을 참조하지
않으므로 user-visible lifetime 없이 안전하게 반환하거나 저장할 수 있다.

## Ownership과 callable ABI

- 모든 function value는 v0.3에서 move-only다. Direct named function identifier를
  value로 평가할 때는 새 function value를 만든다.
- Plain indirect call은 function value를 소비하지 않고 call 동안 읽기 access를
  사용한다.
- Mutable indirect call은 function value를 소비하지 않고 call 동안 exclusive
  access를 사용한다.
- Owned function parameter와 return은 function value를 이동한다. `con`/`mut`
  parameter는 기존 call-scoped borrow 규칙을 재사용한다.
- C backend function value는 typed call thunk, environment pointer, drop thunk를
  가진 내부 value로 표현한다.
- Named function value와 capture 없는 plain closure는 environment 없이 표현할 수
  있다.
- Capturing closure environment는 compiler-owned allocation이며 allocation failure와
  exactly-once cleanup을 기존 runtime error/drop gate로 검증한다.

## 구현 순서

1. function type과 plain function literal token/AST/parser를 추가한다. (완료)
2. named function value, function parameter/return, indirect call semantic을 추가한다. (완료)
3. typed IR와 non-capturing callable C ABI를 추가한다. (완료)
4. immutable owned capture 분석과 environment lowering/cleanup을 추가한다. (완료)
5. `func mut` type/literal과 exclusive call/capture mutation을 추가한다. (완료)
6. package-qualified function value와 public API type linking을 추가한다.
7. positive native smoke, invalid capture/move/alias rejection, sanitizer gate를 추가한다.

## 제외

- borrowed `con`/`mut` capture와 user-visible lifetime
- implicit shared mutable capture
- async function, generator, coroutine
- closure equality, ordering, printing, reflection
- recursive closure와 mutually recursive local function
- variadic function과 function overloading
- dynamic dispatch interface

## 완료 기준

- Function을 parameter와 return value로 전달하고 native 실행한다.
- Immutable owned capture가 Copy/non-copy move 규칙을 따른다.
- Mutable closure는 exclusive access 없이는 호출할 수 없다.
- Escaping closure가 captured non-copy value를 정확히 한 번 정리한다.
- Package-qualified named function과 closure를 representative project에서 사용한다.
- Invalid capture, use-after-move, mutable alias가 source diagnostic으로 거부된다.
- Generated C warning gate와 closure cleanup sanitizer smoke가 통과한다.
