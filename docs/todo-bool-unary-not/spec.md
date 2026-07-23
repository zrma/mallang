# Spec: bool-unary-not

Status: complete; historical milestone record

## 목표

- Mallang v0의 `!expr` bool unary operator를 공식 surface로 고정한다.
- Parser, semantic checker, typed IR, native backend에서 `!bool` 경로를
  regression test로 묶는다.
- `&&` / `||`와 함께 쓰는 precedence를 문서와 native smoke로 검증한다.

## 범위

- Parser:
  - `!expr`를 unary expression으로 parse한다.
  - `!a && b`는 `(!a) && b`로 해석한다.
- Semantic checker:
  - `!` operand는 `bool`이어야 한다.
  - `!` result type은 `bool`이다.
- Backend:
  - typed IR `UnaryOp::Not`을 C `!`로 lower한다.
  - native smoke에서 `!`와 short-circuit operator 조합 결과를 검증한다.
- 이번 slice에서는 user-defined operator overloading이나 truthy/falsy coercion은
  다루지 않는다.

## C-체크리스트

| ID | 상태 | Verify command | 작업 항목 |
| --- | --- | --- | --- |
| C1 | done | `scripts/check.sh` | parser/semantic/backend coverage for `!` |
| C2 | done | `scripts/check.sh` | native smoke for unary not with logical operators |
| C3 | done | `scripts/check.sh` | docs/spec/roadmap/handoff 갱신 |

## 완료 기준

- C-체크리스트가 완료되고 검증 명령이 통과한다.
