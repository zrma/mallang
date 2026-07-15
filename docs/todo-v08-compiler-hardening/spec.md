# Spec: v0.8 Compiler Hardening

상태: Q1-Q6 approved; P161 complete, P162 in progress (Slice A complete, Slice B next)

## Goal

새로운 language surface보다 malformed input에서의 compiler 생존성, 유용한 복수 진단,
deterministic property evidence와 측정 가능한 성능/재현성 baseline을 우선한다.

## Candidate scope

- hand-written parser의 bounded error recovery와 deterministic multiple diagnostics
- user-reachable panic/unchecked invariant audit와 malformed typed IR defense
- lexer/parser/type/ownership deterministic property and crash-corpus tests
- compile latency, generated C size, native binary size와 runtime baseline
- generated C/release archive reproducibility contract
- full LSP와 parser-library migration의 evidence-based deferral

## Implementation order

### P161: Hardening Decision and Baseline Inventory

- Q1-Q6 approval, current panic/recovery/performance inventory와 representative corpus를
  고정한다.

진행:

- [x] Q1-Q6 approval record
- [x] fail-fast lexer/parser/frontend/compiler flow inventory
- [x] production invariant site classification boundary
- [x] property/crash-corpus gap inventory
- [x] representative performance/reproducibility baseline set
- [x] P162 top-level/block/cap slice order

### P162: Parser Recovery and Multiple Diagnostics

- top-level declaration 및 block statement boundary recovery를 구현한다.
- human/JSON diagnostic parity, stable order와 diagnostic cap을 검증한다.

진행:

- [x] Slice A: top-level recovery, multi-source aggregation와 compiler/CLI 연결
- [x] 기존 single-error API와 partial-program rejection 보존
- [x] source별 32-error cap과 human/JSON multi-record parity 회귀
- [ ] Slice B: delimiter-aware block statement recovery
- [ ] Slice C: duplicate suppression, truncation과 compatibility acceptance

### P163: Compiler and IR Invariant Defense

- malformed source로 도달 가능한 panic/unchecked indexing을 stage diagnostic으로 바꾼다.
- typed IR/backend invariant validator와 negative tests를 확장한다.

### P164: Property and Crash-corpus Testing

- deterministic lexer/parser mutation properties와 type/ownership negative corpus를 추가한다.
- discovered regression은 최소 source로 축소해 checked-in corpus에 보존한다.

### P165: Performance and Reproducibility Baseline

- representative projects의 compiler/runtime metrics를 machine-readable record로 남긴다.
- generated C와 release archive의 same-input byte identity 범위를 고정한다.

### P166: v0.8 Acceptance

- crash corpus, full examples, strict C, sanitizer, release binary와 supported-platform CI를
  하나의 hardening evidence로 닫는다.
- documentation과 v0.9 language-freeze decision gate를 동기화한다.

## Excluded

- 새로운 ownership/reference syntax 또는 language feature
- blanket panic suppression을 public diagnostic contract로 삼는 것
- native executable의 cross-toolchain byte identity 보장
- always-on nightly fuzzing service
- full LSP, editor plugin 또는 parser-library rewrite
