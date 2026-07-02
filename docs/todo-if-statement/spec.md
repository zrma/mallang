# Spec: if-statement

## 목표

- Go-like statement-form `if`를 parser, semantic checker, typed IR, C backend에
  end-to-end로 추가한다.

## 범위

- 문법: `if <bool-expr> { <statements> } else { <statements> }`
- `else` 없는 `if <bool-expr> { <statements> }`도 statement로 허용한다.
- branch block 안의 binding은 branch 밖으로 새지 않는다.
- branch 안에서 move된 outer binding은 branch 이후 moved 상태로 보수적으로 합친다.
- `if` expression은 기존처럼 `else`와 branch value를 요구한다.
- `else if` sugar와 block expression generalization은 이번 work unit 범위 밖이다.
- return-completeness analysis는 후속 `docs/todo-return-completeness/spec.md`에서
  statement-form `if` branch 대상으로 확장한다.

## C-체크리스트

| ID | 상태 | Verify command | 작업 항목 |
| --- | --- | --- | --- |
| C1 | done | `cargo test parser::tests::parses_if_statement` | AST/parser에 `StmtKind::If`와 statement parser 추가 |
| C2 | done | `cargo test semantic::tests::if_statement` | semantic branch scope와 moved-state merge 추가 |
| C3 | done | `cargo test ir::tests::ir_lowers_if_statement` | typed IR lowering에 statement `if` 추가 |
| C4 | done | `scripts/check.sh` | C backend codegen, native smoke, 문서 갱신 |

## 완료 기준

- C-체크리스트가 완료되고 검증 명령이 통과한다.
