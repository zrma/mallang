# Spec: array-element-borrow-args

Status: complete; historical milestone record

## 목표

- Fixed-size array의 non-copy element를 owned value로 꺼내지 않고 함수 인자로
  직접 borrow할 수 있게 한다.
- `con values[i]`는 read-only element borrow, `mut values[i]`는 mutable element
  borrow로 고정한다.
- 기존 move safety는 유지한다. `value := values[i]`처럼 non-copy element를 값으로
  indexing하는 경로는 계속 reject한다.

## 범위

- Parser surface는 기존 `con expr` / `mut expr` call argument syntax를 재사용한다.
- Semantic checker:
  - borrow argument place에 direct local array element와 그 field path를 추가한다.
  - index expression은 `int` type과 literal bounds를 기존 indexing 규칙대로 검증한다.
  - mutable element borrow는 root array binding이 `mut`일 때만 허용한다.
  - 같은 call 안에서 같은 array root의 indexed borrow는 conservative하게 overlap으로 본다.
- Typed IR:
  - borrow argument lowering에서 non-copy index expression을 허용한다.
  - owned expression lowering의 non-copy index reject는 유지한다.
- Native backend:
  - borrow arguments는 value-copy expression 대신 addressable lvalue로 lowering한다.
  - array element borrow는 runtime bounds guard를 유지한다.
- 이번 slice에서는 method receiver로 `values[i].method()`를 호출하는 surface,
  borrowed indexing expression 자체, slices `[]T`, append/growth는 다루지 않는다.

## C-체크리스트

| ID | 상태 | Verify command | 작업 항목 |
| --- | --- | --- | --- |
| C1 | done | `scripts/check.sh` | semantic/IR/backend support for array element borrow args |
| C2 | done | `scripts/check.sh` | native smoke for `con values[i]` and `mut values[i]` |
| C3 | done | `scripts/check.sh` | docs/spec/roadmap/handoff 갱신 |

## 완료 기준

- C-체크리스트가 완료되고 검증 명령이 통과한다.
