# Spec: branch-local-cleanup-drop-insertion

## 목표

- Cleanup drop insertion을 straight-line top-level body에서 `if`/`match`
  statement body의 branch-local cleanup roots까지 확장한다.
- Source-level slice surface는 계속 reserved 상태로 둔다.

## 범위

- `if` statement의 then/else body에 empty cleanup scope를 적용한다.
- Statement-form `match` arm body에 empty cleanup scope를 적용한다.
- Branch-local cleanup roots는 arm tail 또는 arm-local `return` 전에
  `IrStmtKind::Drop`으로 삽입한다.
- Nested `if`/`match` statement body도 같은 branch-local 규칙을 재귀 적용한다.

## 제외

- Outer cleanup root가 branch 내부에서 move되는 control-flow cleanup merge.
- `for`, `break`, `continue` 경로의 cleanup insertion.
- Field/index assignment old-value drop insertion.
- Source-level `drop(value)` syntax.
- Slice literal, `len(slice)`, `slice[i]`, `append` 구현.

## C-체크리스트

| ID | 상태 | Verify command | 작업 항목 |
| --- | --- | --- | --- |
| C1 | done | `cargo test --workspace cleanup` | if/match branch-local cleanup insertion regression |
| C2 | done | `scripts/check.sh` | existing native surface and reserved slice boundary 유지 |

## 완료 기준

- C-체크리스트가 완료되고 검증 명령이 통과한다.
