# Spec: branch-outer-cleanup-move-normalization

## 목표

- Cleanup root가 `if`/statement-form `match` branch 안에서 move될 때 parent
  cleanup state가 semantic branch-move merge와 같은 방향으로 정규화되게 한다.
- Source-level slice surface는 계속 reserved 상태로 둔다.

## 범위

- `if` condition에서 move된 active cleanup root를 parent active roots에서 제거한다.
- `if` then/else 중 하나의 continuing branch에서 move된 outer cleanup root는
  move하지 않은 다른 continuing branch tail에서 `IrStmtKind::Drop`으로 정리한다.
- Statement-form `match` scrutinee move와 arm-local outer cleanup root move도 같은
  merge-drop 규칙을 적용한다.
- Branch-local `return` 전에는 branch-local cleanup roots와 outer cleanup roots를
  함께 drop하되, returned root는 제외한다.

## 제외

- `for`, `range`, `break`, `continue` 경로의 cleanup insertion.
- Expression-form `if`/`match` branch move normalization.
- Field/index assignment old-value drop insertion.
- Source-level `drop(value)` syntax.
- Slice literal, `len(slice)`, `slice[i]`, `append` 구현.

## C-체크리스트

| ID | 상태 | Verify command | 작업 항목 |
| --- | --- | --- | --- |
| C1 | done | `cargo test --workspace cleanup` | if/match outer cleanup root branch move regression |
| C2 | done | `scripts/check.sh` | existing native surface and reserved slice boundary 유지 |

## 완료 기준

- C-체크리스트가 완료되고 검증 명령이 통과한다.
