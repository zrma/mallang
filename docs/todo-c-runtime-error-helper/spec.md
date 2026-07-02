# Spec: c-runtime-error-helper

## 목표

- Generated C의 Mallang runtime failure emission을 한 helper로 모은다.
- Integer overflow/division, index bounds, slice allocation failure guard가 같은
  error surface를 사용하게 한다.
- Future backend runtime checks가 같은 helper를 재사용하도록 경계를 만든다.

## 결정

- Generated C prelude에 `mallang_runtime_error(const char *message)`를 둔다.
- Guard emitters는 `fprintf`와 `exit`를 직접 생성하지 않고
  `mallang_runtime_error("message")` 호출을 생성한다.
- Runtime error prefix는 helper가 소유한다.

## 범위

- C backend runtime helper prelude 추가.
- Expression/statement guard emission의 runtime failure path 통합.
- Backend regression: generated C direct `fprintf(stderr, ...)`는 helper 하나만
  포함한다.

## 제외

- Runtime error recovery.
- Source span-aware runtime diagnostics.
- Non-runtime compile error message 변경.

## C-체크리스트

| ID | 상태 | Verify command | 작업 항목 |
| --- | --- | --- | --- |
| C1 | done | `cargo test --all-targets backend::c::tests::generates_c` | C backend runtime helper regression |
| C2 | done | `scripts/check.sh` | full native smoke 유지 |
