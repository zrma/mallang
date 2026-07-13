# Open Questions: v0.3-functions-closures

상태: awaiting user decision

## Q1. Function type 문법

추천: `func(int) int`, `func(con string) int`, `func mut(int) int`

- Go-like declaration과 가장 가깝고 parameter mode를 ABI contract에 보존한다.
- `func mut`은 capture 변경 가능성을 function type에 명시한다.

대안:

- `Fn(int) -> int` / `FnMut(int) -> int`: 의미는 선명하지만 Go-like surface에서
  크게 벗어난다.
- `(int) -> int`: 짧지만 기존 declaration 문법과 별도 type 문법이 생긴다.

## Q2. Closure literal 문법

추천: `func(value int) int { ... }`, `func mut(delta int) int { ... }`

- Named function과 parameter/return 문법을 그대로 재사용한다.
- Parser는 `func` 다음 token이 identifier/receiver인지 `(`/`mut`인지로 declaration과
  literal을 구분할 수 있다.

대안:

- Rust형 `|value| value * 2`: 간결하지만 parameter type/mode와 block 규칙을 새로
  정의해야 한다.
- `fn(value int) int { ... }`: declaration과 literal keyword가 달라진다.

## Q3. Capture 방식

추천: capture list 없이 free local을 owned-by-value로 자동 capture한다.

- Copy는 복사하고 non-copy는 이동하므로 현재 ownership 규칙과 일치한다.
- Closure가 stack borrow를 보관하지 않아 escape가 안전하다.

대안:

- `[move value, con name, mut count]`: 명시적이지만 first-class reference와 lifetime
  문제를 v0.3으로 끌어온다.
- Rust처럼 context에 따라 borrow 또는 move를 추론: 편리하지만 user-visible
  lifetime 없이 escape diagnostic을 안정적으로 설명하기 어렵다.

## Q4. Mutable capture

추천: `func mut`으로 call effect를 type에 넣고 mutable closure 호출에 exclusive
access를 요구한다.

- 기존 mutable binding과 `mut` borrow 규칙으로 alias safety를 검증할 수 있다.
- Plain function과 stateful closure의 API contract가 signature에 남는다.

대안:

- 모든 closure를 mutable callable로 취급: 단순하지만 pure higher-order code도 항상
  mutable storage를 요구한다.
- Mutability를 type에서 숨기고 body로만 추론: package/API 경계에서 call 권한을
  표현할 수 없다.

## Q5. v0.3 capture 범위

추천: owned capture만 구현하고 borrowed capture는 v0.5 memory-model gate까지
보류한다.

- Escaping closure를 안전하게 먼저 완성할 수 있다.
- `con`/`mut`은 closure value를 함수에 전달할 때의 call-scoped borrow로 계속
  사용할 수 있다.

대안:

- Non-escaping closure만 borrowed capture 허용: escape analysis와 lifetime-like
  diagnostic이 먼저 필요하다.

## 승인 요청

Q1-Q5 추천안을 함께 승인하면 `spec.md`의 surface를 v0.3 구현 계약으로 확정한다.
수정이 필요한 항목이 있으면 해당 질문만 다시 연다.
