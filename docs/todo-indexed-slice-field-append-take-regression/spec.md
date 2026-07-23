# Spec: indexed-slice-field-append-take-regression

Status: complete; historical milestone record

## 목표

- Indexed owned slice field source를 `append` source로 take하는 C lowering을 regression으로 고정한다.
- `append(store.bags[i].values, item)`가 consumed buffer를 result로 넘기고 source field를 empty slice로 reset하는지 확인한다.
- Existing native smoke coverage를 backend unit level에서도 증명한다.

## 결정

- `generate_c_from_ir` backend test에 indexed field append-take source case를 추가한다.
- C output에서 indexed source lvalue copy, empty source reset, result ownership, final store cleanup을 확인한다.
- 새 surface syntax는 열지 않는다. 이미 허용된 P59 behavior의 backend proof를 보강한다.

## 범위

- C backend regression for indexed slice field append-take source.
- ROADMAP/HANDOFF 갱신.

## 제외

- Full C AST assertion helper.
- Same-field indexed append reassignment behavior 변경.
- Runtime output smoke 추가. 기존 `examples/slice-field-take-append.mlg`가 이미 indexed source take를 포함한다.

## C-체크리스트

| ID | 상태 | Verify command | 작업 항목 |
| --- | --- | --- | --- |
| C1 | done | `cargo test --all-targets backend::c::tests::generates_c_for_indexed_slice_field_append_take_source` | indexed field append-take backend regression |
| C2 | done | `scripts/check.sh` | existing native indexed source take smoke 유지 |
