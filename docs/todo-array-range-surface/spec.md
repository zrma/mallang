# Spec: array-range-surface

## 목표

- Mallang v0의 array/slice/range 최소 surface를 구현 전에 확정한다.
- 다음 구현 slice가 parser, semantic, IR, backend를 같은 목표로 향하게 한다.

## 범위

- Fixed-size array를 첫 구현 대상으로 삼는다.
- Type syntax는 Go-like `[N]T`로 한다.
- Literal syntax는 `[N]T{a, b, c}`로 한다.
- `N`은 non-negative integer literal이다.
- Array literal은 element를 정확히 `N`개 제공해야 한다.
- Arrays are move-only by default in v0.
- Native layout은 raw C array가 아니라 struct-wrapper `data[N]` layout을 쓴다.
  그래야 array value를 기존 value pipeline에서 assign/move/pass 할 수 있다.
- `range`의 첫 구현은 array-only `for i, value := range values { ... }`다.
- Range loop는 immutable `int` index와 immutable element value를 body-local
  binding으로 도입한다.
- 첫 range slice에서 element binding은 `Copy` element type만 허용한다.
- Range source는 read-only iteration이고 loop 뒤에도 사용할 수 있다.
- Slices `[]T`, indexing, `len`, append/growth, blank identifier, one-variable
  range, mutable range, by-reference element iteration은 후속 slice로 미룬다.

## C-체크리스트

| ID | 상태 | Verify command | 작업 항목 |
| --- | --- | --- | --- |
| C1 | done | `scripts/check.sh` | SPEC에 fixed-size array와 array-only `range` surface 결정 기록 |
| C2 | done | `scripts/check.sh` | ROADMAP/docs/HANDOFF에 다음 implementation slice 순서 기록 |

## 완료 기준

- C-체크리스트가 완료되고 검증 명령이 통과한다.
