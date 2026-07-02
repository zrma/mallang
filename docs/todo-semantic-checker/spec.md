# Spec: semantic-checker

## 목표

- `mlg check` subcommand와 별도 semantic phase를 추가해 parser 이후 오류를
  backend 전에 진단한다.

## 범위

- 함수 중복 선언 검출.
- `main` 선언 존재와 parameter 없음 검증.
- local binding name resolution.
- unresolved identifier 진단.
- `nil` 사용 진단.
- function call arity/type mismatch 진단.
- first native subset의 primitive type checking을 semantic phase로 이동.
- backend는 semantic phase 성공 후에만 실행.
- ownership/move/borrow rule은 다음 milestone로 남긴다.

## C-체크리스트

| ID | 상태 | Verify command | 작업 항목 |
| --- | --- | --- | --- |
| C1 | done | `scripts/check.sh` | semantic checker와 `mlg check` 구현 |
| C2 | done | `cargo test --workspace semantic` | semantic diagnostics 회귀 테스트 |

## 완료 기준

- C-체크리스트가 완료되고 검증 명령이 통과한다.

## 검증 증거

- `scripts/check.sh`
- `cargo test --workspace semantic`
