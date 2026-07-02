# Spec: non-copy-array-element-assignment

## 목표

- Fixed-size array의 non-copy element도 mutable slot assignment로 교체할 수 있게 한다.
- `values[i] = expr`는 element type이 `Copy`인지와 무관하게 type이 맞는 owned
  value를 배열 슬롯에 저장한다.
- non-copy element를 값으로 꺼내는 indexing expression은 계속 금지한다.

## 범위

- Parser surface는 기존 `values[i] = expr`와 `for ; ; values[i] = expr` post
  syntax를 재사용한다.
- Semantic checker:
  - root array binding/parameter가 `mut`이어야 한다.
  - index는 기존 `int` type/literal bounds 규칙을 따른다.
  - RHS는 owned value로 type-check하며 non-copy RHS는 move된다.
- Typed IR:
  - statement-form index assignment에서 non-copy element를 허용한다.
  - for-post assignment target lowering도 non-copy index target을 허용한다.
- Native backend:
  - 기존 checked slot assignment lowering을 non-copy element에도 사용한다.
- 이번 slice에서는 non-copy element indexing expression, destructuring, drop/destructor
  semantics, slice `[]T`는 다루지 않는다.

## C-체크리스트

| ID | 상태 | Verify command | 작업 항목 |
| --- | --- | --- | --- |
| C1 | done | `scripts/check.sh` | semantic/IR support for non-copy element assignment |
| C2 | done | `scripts/check.sh` | native smoke for struct element replacement |
| C3 | done | `scripts/check.sh` | docs/spec/roadmap/handoff 갱신 |

## 완료 기준

- C-체크리스트가 완료되고 검증 명령이 통과한다.
