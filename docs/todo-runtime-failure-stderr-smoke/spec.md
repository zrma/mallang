# Spec: runtime-failure-stderr-smoke

## 목표

- Native runtime failure smoke가 exit code뿐 아니라 Mallang runtime stderr message도 검증한다.
- P72 runtime error helper가 실제 `mlg run` 실패 경로에서 사용자-visible message를 유지하는지 확인한다.
- Compile-time check failure smoke와 runtime failure smoke의 검증 경계를 분리한다.

## 결정

- `expect_native_runtime_failure label source expected_stderr` helper를 `scripts/check.sh`에 둔다.
- Runtime negative smoke는 `mlg run` stderr에서 `mallang runtime error: ...` substring을 확인한다.
- Semantic/compile-time negative smoke는 기존처럼 non-zero check로 유지한다.

## 범위

- Division/remainder by zero runtime failure stderr.
- Checked integer overflow runtime failure stderr.
- Array bounds runtime failure stderr.
- ROADMAP/HANDOFF 갱신.

## 제외

- Full CLI integration test harness.
- Source span-aware runtime diagnostics.
- Compile-time diagnostic stderr assertions.

## C-체크리스트

| ID | 상태 | Verify command | 작업 항목 |
| --- | --- | --- | --- |
| C1 | done | `scripts/check.sh` | native runtime stderr smoke |
| C2 | done | `cargo clippy --all-targets -- -D warnings` | Rust lint 유지 |
