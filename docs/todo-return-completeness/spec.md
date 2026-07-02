# Spec: return-completeness

## 목표

- non-`unit` function의 return-completeness analysis를 statement-form `if`
  branch까지 확장한다.
- `if/else` 양쪽 branch가 모두 return하면 function-level return으로 인정한다.

## 범위

- 허용: top-level `return`.
- 허용: `if cond { return ... } else { return ... }`.
- 허용: nested statement-form `if`가 모든 branch에서 return하는 경우.
- 거부: `if`에 `else`가 없어서 일부 path가 return하지 않는 경우.
- 거부: `else` branch가 return하지 않는 경우.
- 제외: `else if` parser sugar.
- 제외: statement-block `match` arm return analysis.

## C-체크리스트

| ID | 상태 | Verify command | 작업 항목 |
| --- | --- | --- | --- |
| C1 | done | `cargo test semantic::tests::` | branch-aware return-completeness semantic test 추가 |
| C2 | done | `scripts/check.sh` | `examples/return-completeness.mlg` native smoke 추가 |
| C3 | done | `scripts/check.sh` | README/SPEC/ROADMAP/HANDOFF 상태 갱신 |

## 완료 기준

- C-체크리스트가 완료되고 검증 명령이 통과한다.
