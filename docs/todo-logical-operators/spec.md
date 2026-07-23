# Spec: logical-operators

Status: complete; historical milestone record

## 목표

- Mallang v0 native subset에 `bool` logical operators `&&`와 `||`를 추가한다.
- Go/C 계열과 같은 precedence와 short-circuit 실행 의미를 smoke로 고정한다.

## 범위

- Lexer/token model에 `&&`와 `||` 토큰을 추가한다.
- AST/parser에 `BinaryOp::LogicalAnd`와 `BinaryOp::LogicalOr`를 추가한다.
- Semantic checker는 양쪽 operand가 `bool`일 때만 허용하고 result type을 `bool`로 본다.
- Typed IR와 C backend는 logical op result를 `bool`로 낮추고 C `&&` / `||`를 emit한다.
- `examples/logical-operators.mlg`로 precedence와 short-circuit side effect를 검증한다.

## C-체크리스트

| ID | 상태 | Verify command | 작업 항목 |
| --- | --- | --- | --- |
| C1 | done | `scripts/check.sh` | `&&` / `||` lexer, parser, semantic, IR, backend, native smoke 추가 |

## 완료 기준

- `scripts/check.sh`가 통과한다.
- `examples/logical-operators.mlg` native output이 short-circuit side effect를 증명한다.
