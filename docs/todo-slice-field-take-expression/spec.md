# Spec: slice-field-take-expression

## 목표

- Owned value position에서 owned slice field를 직접 take할 수 있게 한다.
- Full partial-move tracking 없이 source field를 empty slice로 되돌리는 기존
  compiler-owned take semantics를 append 외부로 확장한다.

## 범위

- Semantic:
  - `taken := bag.values`를 허용한다.
  - `consume(bag.values)`처럼 owned parameter argument에서 slice field source를
    허용한다.
  - `store.bags[i].values`처럼 local-rooted indexed field source도 허용한다.
  - Non-slice non-copy field move는 기존처럼 reject한다.
- IR:
  - owned slice field take를 explicit IR node로 낮춰 read-only field source와
    구분한다.
  - `len(bag.values)`, `bag.values[i]`, `range bag.values`는 read-only source로
    유지하고 take node를 만들지 않는다.
- Backend:
  - source lvalue를 temp slice header로 copy한다.
  - source field에 empty slice header를 write한다.
  - result value나 callee가 consumed buffer를 소유하고, owning struct cleanup은
    empty source field를 drop한다.
- Native smoke:
  - `examples/slice-field-take.mlg`가 owned take, owned argument take, indexed
    field take, source empty reset을 검증한다.

## 제외

- General field partial moves where the source field is left uninitialized.
- Moving non-slice fields out of structs.
- First-class references and statement-spanning borrow lifetimes.

## C-체크리스트

| ID | 상태 | Verify command | 작업 항목 |
| --- | --- | --- | --- |
| C1 | done | `cargo test --workspace allows_owned_slice_field_take_expression` | semantic owned slice field take 허용 |
| C2 | done | `cargo test --workspace ir_lowers_owned_slice_field_take_expression` | IR take/read source 분리 |
| C3 | done | `cargo test --workspace generates_c_for_owned_slice_field_take_expression` | C backend source field empty reset |
| C4 | done | `cargo run --bin mlg -- run examples/slice-field-take.mlg` | native owned slice field take smoke |
| C5 | done | `scripts/check.sh` | full repo smoke includes owned slice field take example |
