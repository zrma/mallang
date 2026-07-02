# Spec: statement-spanning-borrows-decision

## 목표

- v0에서 statement-spanning borrow syntax를 열지 않는다.
- `con expr` / `mut expr`는 call argument mode prefix로만 유지한다.
- First-class reference values는 별도 lifetime model이 생긴 뒤에 연다.

## 결정

- `borrowed := con user.name`는 v0 syntax가 아니다.
- `return mut name`도 v0 syntax가 아니다.
- Borrow duration은 v0에서 function call 하나로 제한된다.
- Borrowed values는 local binding, return value, field storage, ADT payload로
  저장할 수 없다.

## 범위

- Parser regression: borrow marker in let value position reject.
- Parser regression: borrow marker in return value position reject.
- SPEC/roadmap/handoff 갱신.

## 제외

- First-class reference type.
- Statement-spanning borrow lifetime checker.
- Borrowed return values.
- Borrowed fields or ADT payloads.

## C-체크리스트

| ID | 상태 | Verify command | 작업 항목 |
| --- | --- | --- | --- |
| C1 | done | `cargo test --all-targets borrow_marker` | parser borrow marker value-position regression |
| C2 | done | `scripts/check.sh` | full native smoke 유지 |
