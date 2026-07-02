# Spec: match-expression-prelude

## 목표

- `match` expression arm 안에 `if`/`match` 조합처럼 C prelude statement가 필요한 expression이 들어와도 native backend가 코드를 생성하게 한다.
- 단순 `match` expression은 기존 ternary lowering을 유지한다.

## 범위

- Statement-aware `match` expression lowering에서 arm prelude가 비어 있으면 기존 ternary를 유지한다.
- arm prelude가 있으면 C local temp를 만들고 ADT tag 조건별 block에서 temp에 arm 값을 대입한다.
- `match` arm 안의 `if` expression이 다시 non-local `match` expression을 포함하는 native smoke를 추가한다.
- Pure C expression lowering 전체를 statement-aware로 재설계하지는 않는다.

## C-체크리스트

| ID | 상태 | Verify command | 작업 항목 |
| --- | --- | --- | --- |
| C1 | done | `scripts/check.sh` | match expression arm prelude temp lowering 추가 |
| C2 | done | `scripts/check.sh` | nested if/match expression native smoke 추가 |

## 완료 기준

- C-체크리스트가 완료되고 검증 명령이 통과한다.
