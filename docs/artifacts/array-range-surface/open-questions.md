# Open Questions

현재 구현을 막는 미결 항목 없음.

후속 slice에서 별도로 결정할 항목:

- Slice `[]T`의 ownership, borrow, native ABI.
- Indexing expression과 bounds checking 정책.
- `len` builtin을 syntax로 둘지 builtin function으로 둘지.
- Non-copy element range를 move로 볼지 borrow로 볼지.
- Mutable/by-reference range binding syntax.
