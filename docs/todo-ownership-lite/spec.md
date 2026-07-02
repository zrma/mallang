# Spec: ownership-lite

## 목표

- v0 ownership-lite의 첫 정적 규칙을 semantic checker에 추가한다.

## 범위

- `int`, `bool`은 `Copy`로 취급한다.
- `string`은 move-only로 취급한다.
- non-copy local을 owned value position에서 사용하면 moved 상태로 표시한다.
- moved local 재사용을 reject한다.
- `in` parameter와 `in` call argument를 읽기 borrow로 검증한다.
- `mut` parameter와 `mut` call argument를 mutable borrow로 검증한다.
- borrow argument는 v0에서 direct local variable만 허용한다.
- borrow value 저장/return은 별도 first-class reference가 없으므로 이번 범위에서는 생기지 않게 유지한다.

## C-체크리스트

| ID | 상태 | Verify command | 작업 항목 |
| --- | --- | --- | --- |
| C1 | done | `scripts/check.sh` | ownership-lite checker 구현 |
| C2 | done | `cargo test --workspace ownership` | move/borrow diagnostics 회귀 테스트 |

## 완료 기준

- C-체크리스트가 완료되고 검증 명령이 통과한다.

## 검증 증거

- `scripts/check.sh`
- `cargo test --workspace ownership`
