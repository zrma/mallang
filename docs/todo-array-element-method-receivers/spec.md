# Spec: array-element-method-receivers

## 목표

- Fixed-size array element를 struct method receiver로 직접 호출할 수 있게 한다.
- `users[i].method()`가 `con self T` receiver이면 checked read borrow로, `mut self T`
  receiver이면 checked mutable borrow로 lowering된다.
- non-copy array element value extraction은 계속 금지한다.

## 범위

- Parser surface는 기존 method call syntax와 index expression syntax를 재사용한다.
- Semantic checker:
  - method lookup은 direct local/field/index receiver place의 borrowed type으로 수행한다.
  - `con` receiver는 direct local/field/index place를 shared borrow로 검증한다.
  - `mut` receiver는 direct local/field/index place를 exclusive borrow로 검증하고,
    root binding이 `mut`이어야 한다.
  - receiver borrow는 같은 call의 explicit arguments와 overlap check를 공유한다.
  - owned receiver로 non-copy array element를 move하는 것은 허용하지 않는다.
- Typed IR:
  - `con`/`mut` receiver lowering은 existing borrow argument lowering을 재사용한다.
- Native backend:
  - existing checked lvalue address lowering을 receiver argument에도 사용한다.
- 이번 slice에서는 method values, first-class borrowed indexing expressions, slices
  `[]T`, statement-spanning borrow lifetime은 다루지 않는다.

## C-체크리스트

| ID | 상태 | Verify command | 작업 항목 |
| --- | --- | --- | --- |
| C1 | done | `scripts/check.sh` | semantic/IR support for array element method receivers |
| C2 | done | `scripts/check.sh` | native smoke for mutable array element receiver |
| C3 | done | `scripts/check.sh` | docs/spec/roadmap/handoff 갱신 |

## 완료 기준

- C-체크리스트가 완료되고 검증 명령이 통과한다.
