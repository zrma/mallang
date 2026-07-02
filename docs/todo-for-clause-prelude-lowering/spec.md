# Spec: for-clause-prelude-lowering

## 목표

- Native backend에서 `for init; condition; post` clause의 condition/post가
  temporary prelude statement를 필요로 해도 C로 lowering할 수 있게 한다.
- post가 있는 `for` clause에서 `continue`가 Go/C semantics처럼 post를 실행한 뒤
  다음 condition check로 넘어가게 한다.

## 범위

- C `for` header 직접 출력 대신 scoped block + `while (true)` lowering 사용.
- condition prelude를 loop top에서 실행하고 false면 `break`.
- post assignment target/RHS prelude를 post label block 안에 출력.
- body 안 `continue`는 현재 for-clause post label로 `goto` lowering.
- nested loop의 `continue`는 바깥 post label을 상속하지 않음.
- `if`/statement `match` arm body 안의 `continue`는 현재 loop context를 보존.
- `examples/for-clause-prelude.mlg` native smoke 추가.

## C-체크리스트

| ID | 상태 | Verify command | 작업 항목 |
| --- | --- | --- | --- |
| C1 | done | `scripts/check.sh` | condition/post prelude lowering 구현 |
| C2 | done | `scripts/check.sh` | post-preserving `continue` lowering 검증 |
| C3 | done | `scripts/check.sh` | docs/examples/roadmap 갱신 |

## 완료 기준

- C-체크리스트가 완료되고 검증 명령이 통과한다.
