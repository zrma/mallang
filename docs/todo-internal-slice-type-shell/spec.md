# Spec: internal-slice-type-shell

## 목표

- Owned slice 구현 전 단계로 internal type/backend shell을 추가한다.
- User-facing `[]T`는 계속 semantic reserved diagnostic으로 막는다.

## 범위

- `Type::Slice(Box<Type>)`를 내부 타입으로 추가한다.
- Slice type은 `Copy`가 아니며 cleanup resource로 분류한다.
- C backend는 internal slice type을 `{ data, len, cap }` typedef로 emit한다.
- Semantic `type_from_ref`는 여전히 source-level `[]T`를 reserved-feature
  diagnostic으로 reject한다.

## 제외

- Slice literal, `len(slice)`, `slice[i]`, `append` 구현.
- 실제 cleanup/drop statement lowering.
- User-facing slice value construction.

## C-체크리스트

| ID | 상태 | Verify command | 작업 항목 |
| --- | --- | --- | --- |
| C1 | done | `cargo test --workspace slice` | internal slice type/backend shell regression |
| C2 | done | `scripts/check.sh` | reserved `[]T` source surface 유지 |

## 완료 기준

- C-체크리스트가 완료되고 검증 명령이 통과한다.
