# Spec: local-rooted-slice-field-reads

## 목표

- Struct cleanup으로 열린 `[]T` fields를 read/borrow surface에서 직접 사용할 수 있게 한다.
- Slice source 제약을 direct local binding에서 local-rooted place로 넓힌다.
- Inline slice temporaries는 cleanup lifetime model이 생길 때까지 계속 거부한다.

## 범위

- Semantic:
  - `len(bag.values)`처럼 local-rooted slice place의 length read를 허용한다.
  - `bag.values[i]`처럼 local-rooted slice place의 Copy element value read를 허용한다.
  - `for _, value := range bag.values`처럼 local-rooted slice place range를 허용한다.
  - `con bag.values[i]` / `mut bag.values[i]` borrow argument를 허용한다.
- IR/backend:
  - 기존 field/index expression lowering과 hidden-reference borrow lowering을 재사용한다.
- Native smoke:
  - `examples/slice-field-read.mlg`에서 len/index/range/borrow read와 mutable element borrow를 검증한다.

## 제외

- `append(bag.values, item)`처럼 field slice를 consuming append source로 쓰는
  경로는 P57/P58에서 same-field reassignment로 제한해 완료됐다.
- `bag.values[i] = item`처럼 direct element assignment target으로 쓰는 경로.
  이 경로는 P56에서 완료됐다.
- Inline cleanup temporary sources such as `len([]int{1})`.
- First-class references and statement-spanning borrow lifetimes.

## C-체크리스트

| ID | 상태 | Verify command | 작업 항목 |
| --- | --- | --- | --- |
| C1 | done | `cargo test --workspace allows_local_rooted_slice_field_read_sources` | semantic read/range/borrow 허용 |
| C2 | done | `cargo run --bin mlg -- run examples/slice-field-read.mlg` | native local-rooted slice field smoke |
| C3 | done | `scripts/check.sh` | full repo smoke includes slice field read example |
