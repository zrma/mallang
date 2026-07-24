# Spec: slice-reserved-boundary

Status: complete; historical milestone record

## 목표

- slice ownership/native ABI를 열기 전까지 `[]T`가 모든 type position에서
  같은 reserved-feature boundary를 유지하도록 고정한다.
- future `append`/slice work가 시작될 때 풀어야 하는 표면을 명확히 남긴다.

## 범위

- Direct function parameter `[]T` reserved diagnostic을 유지한다.
- Function return type, struct field, generic payload, fixed-array element type에
  포함된 `[]T`도 semantic checker에서 reserved diagnostic으로 reject한다.
- Parser의 `[]T` syntax support와 fixed-size array `[N]T` support는 변경하지
  않는다.

## C-체크리스트

| ID | 상태 | Verify command | 작업 항목 |
| --- | --- | --- | --- |
| C1 | done | `cargo test --workspace slice_type_refs` | nested type position semantic regression |
| C2 | done | `scripts/check.sh` | nested slice reserved failure smoke |
| C3 | done | `docs/ROADMAP.md` / `docs/HANDOFF.md` | reserved boundary 문서화 |

## 완료 기준

- C-체크리스트가 완료되고 검증 명령이 통과한다.
