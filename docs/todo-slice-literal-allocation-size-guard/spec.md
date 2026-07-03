# Spec: slice-literal-allocation-size-guard

## 목표

- Native slice literal lowering이 `malloc(sizeof(T) * len)` 전에 allocation-size overflow를 guard한다.
- Slice literal과 `append`가 같은 runtime-error allocation policy를 갖게 한다.
- Native backend memory safety audit에서 확인된 unchecked allocation-size multiplication gap을 닫는다.

## 결정

- Non-empty slice literal C prelude에 `UINT64_MAX / sizeof(T)` guard를 추가한다.
- Overflow와 allocation failure는 기존 `mallang_runtime_error(...)` helper를 사용한다.
- Empty slice literal은 allocation하지 않고 null data pointer, zero len/cap으로 유지한다.

## 범위

- C backend slice literal allocation-size guard.
- Backend codegen regression.
- SPEC/ROADMAP/HANDOFF 갱신.

## 제외

- Full allocator abstraction.
- Compile-time maximum slice literal length policy.
- Runtime allocation failure injection test harness.

## C-체크리스트

| ID | 상태 | Verify command | 작업 항목 |
| --- | --- | --- | --- |
| C1 | done | `cargo test --all-targets backend::c::tests::generates_c_for_slice_literal_indexing_len_and_cleanup` | slice literal allocation-size guard codegen |
| C2 | done | `scripts/check.sh` | native smoke 유지 |
