# Spec: slice-ownership-append-decision

## 목표

- `[]T`, `append`, range over slices를 열기 전 ownership/native ABI 결정을
  고정한다.
- Go-like syntax를 유지하되 Go-style shared backing-array aliasing을 v0에
  들이지 않는다.

## 결정

- `[]T`는 owned, move-only growable buffer다.
- Slice value는 native에서 compiler-owned `{ data, len, cap }` header로
  lowering한다.
- Empty slice는 `data = null`, `len = 0`, `cap = 0`을 허용한다.
- Slice header copy는 언어 operation이 아니다. Assignment, parameter passing,
  return은 기존 non-copy value처럼 move로 처리한다.
- `append(values, item)`은 built-in이다. 첫 인자인 owned slice와 item을
  소비하고 새 owned slice를 반환한다.
- Caller-visible update는 `mut values` local에 `values = append(values, item)`
  형태로 표현한다.
- `len(values)`는 fixed-size array와 owned slice 양쪽에서 read-only로 동작하며
  값을 move하지 않는다.
- `slice[i]` value access는 fixed-size array와 같이 Copy element에 먼저
  한정한다.
- `con slice[i]` / `mut slice[i]`는 기존 element borrow surface를 확장하되,
  slice root alias/overlap rule이 구현된 뒤에 연다.

## 구현 순서

1. IR/backend에 owned heap resource cleanup/drop lowering을 추가한다.
2. Semantic `Type::Slice(Box<Type>)`와 IR/backend type shell을 추가하되,
   정상 value construction은 아직 제한한다. 이 shell은 P34에서 완료됐다.
3. `[]T{...}` literal, `len(slice)`, Copy-only `slice[i]` value access를
   구현한다. 이 단계는 P47에서 완료됐다.
4. consuming `append(values, item)` built-in을 구현한다. 이 단계는 P48에서
   완료됐다.
5. slice range와 element borrow를 별도 slice로 확장한다.

## 제외

- Borrowed slice view as first-class value.
- Multiple owned slice values sharing one backing allocation.
- Mutable range value and by-reference range iteration.
- Statement-spanning borrow lifetime syntax.

## C-체크리스트

| ID | 상태 | Verify command | 작업 항목 |
| --- | --- | --- | --- |
| C1 | done | `scripts/check.sh` | SPEC/README/HANDOFF decision update |
| C2 | done | `docs/ROADMAP.md` | P33 decision milestone 기록 |

## 완료 기준

- C-체크리스트가 완료되고 검증 명령이 통과한다.
