# Spec: cleanup-drop-helper-shell

## 목표

- Owned heap resource cleanup/drop lowering의 선행 단계로 C backend drop helper
  shell을 추가한다.
- Helper emission은 internal cleanup types에만 적용하고, source-level `[]T`
  surface는 계속 reserved 상태로 둔다.

## 범위

- Cleanup type별 `static void mlg_drop_<Type>(<Type> *mlg_value)` helper를
  emit한다.
- `Type::Slice(T)` helper는 `mlg_data`를 `free`하고 slice header를 null/zero로
  reset한다.
- `Option[T]`, `Result[T, E]`, `[N]T` wrapper helper는 active payload 또는
  element가 cleanup type일 때 해당 payload/element helper를 호출한다.

## 제외

- Scope exit, early return, reassignment 지점에 실제 drop call을 삽입하는
  lifetime lowering.
- Slice literal, `len(slice)`, `slice[i]`, `append` 구현.
- User-facing slice value construction.

## C-체크리스트

| ID | 상태 | Verify command | 작업 항목 |
| --- | --- | --- | --- |
| C1 | done | `cargo test --workspace cleanup` | internal cleanup helper codegen regression |
| C2 | done | `scripts/check.sh` | existing native surface and reserved slice boundary 유지 |

## 완료 기준

- C-체크리스트가 완료되고 검증 명령이 통과한다.
