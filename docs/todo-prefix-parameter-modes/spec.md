# Spec: prefix-parameter-modes

## 목표

- Borrow parameter/call surface를 `con`/`mut` prefix mode로 정리한다.
- `con name T`는 const/read-only borrow, `mut name T`는 mutable borrow,
  아무 mode가 없으면 owned value로 고정한다.
- 기존 `in` borrow keyword와 `name in T` / `name mut T` suffix surface는 v0
  PoC에서 제거한다.

## 범위

- Lexer: `con` keyword 추가, `in` borrow keyword 제거.
- Parser: parameter/receiver mode를 `con name T` / `mut name T` prefix로 parse.
- Parser: call argument mode를 `con expr` / `mut expr`로 parse.
- Semantic diagnostics: read-borrow argument mismatch를 `con` 기준으로 표시.
- Examples/docs/tests: borrow surface를 canonical prefix syntax로 갱신.
- 내부 enum 이름 `ParamMode::In` / `ArgMode::In`은 구현 세부로 유지한다.

## C-체크리스트

| ID | 상태 | Verify command | 작업 항목 |
| --- | --- | --- | --- |
| C1 | done | `cargo test --workspace` | parser/semantic/backend coverage |
| C2 | done | `scripts/check.sh` | examples native smoke with `con` syntax |

## 완료 기준

- C-체크리스트가 완료되고 검증 명령이 통과한다.
