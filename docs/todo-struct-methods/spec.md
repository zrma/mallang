# Spec: struct-methods

## 목표

- Go-like receiver method declaration과 method call을 parser, semantic checker,
  typed IR, C backend까지 end-to-end로 추가한다.

## 범위

- Declaration syntax: `func (self in User) age() int { ... }`
- Call syntax: `user.age()`
- Receiver는 기존 param mode를 재사용한다: owned, `in`, `mut`.
- 이번 native smoke는 read receiver `self in User`와 copy field 반환을 검증한다.
- Method lowering은 내부적으로 receiver를 첫 번째 인자로 받는 static function으로
  변환한다.
- non-copy field return, field assignment, receiver-specific overload beyond struct
  receiver, method values, interface dispatch는 범위 밖이다.

## C-체크리스트

| ID | 상태 | Verify command | 작업 항목 |
| --- | --- | --- | --- |
| C1 | done | `cargo test parser::tests::parses_method_declaration_and_call` | AST/parser receiver method declaration과 method call 추가 |
| C2 | done | `cargo test semantic::tests::allows_read_receiver_method_call` | semantic method signature, receiver mode, method lookup 추가 |
| C3 | done | `cargo test ir::tests::ir_lowers_method_declarations_and_calls` | typed IR method lowering과 method call lowering 추가 |
| C4 | done | `scripts/check.sh` | C backend mangling/native smoke와 문서 갱신 |

## 완료 기준

- C-체크리스트가 완료되고 검증 명령이 통과한다.
