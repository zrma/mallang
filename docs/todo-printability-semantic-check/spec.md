# Spec: printability-semantic-check

Status: complete; historical milestone record

## 목표

- `print`가 native backend에서 출력할 수 없는 타입을 `mlg check` 단계에서 reject한다.
- Backend-only print errors를 semantic diagnostic으로 앞당긴다.

## 범위

- Semantic checker:
  - `print` 가능 타입을 `int`, `bool`, `string`, printable payload의 `Option`,
    printable payload의 `Result`, printable fields의 `struct`로 고정한다.
  - `unit`, fixed-size arrays, fixed-size array를 포함한 `Option`/`Result`/`struct`
    print를 reject한다.
  - recursive struct value type은 이미 semantic에서 reject하므로 printability
    traversal은 cycle-safe assumption 아래 구현한다.
- Backend:
  - 기존 backend invariant guard는 유지한다.
- Smoke:
  - array print와 array field struct print가 `mlg check`에서 non-zero로
    실패하는지 검증한다.

## C-체크리스트

| ID | 상태 | Verify command | 작업 항목 |
| --- | --- | --- | --- |
| C1 | done | `scripts/check.sh` | array/unit print semantic reject 유지 |
| C2 | done | `scripts/check.sh` | non-printable Option/Result/struct print semantic reject |
| C3 | done | `scripts/check.sh` | docs/spec/roadmap/handoff 갱신 |

## 완료 기준

- C-체크리스트가 완료되고 검증 명령이 통과한다.
