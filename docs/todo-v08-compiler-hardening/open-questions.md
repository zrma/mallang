# Open Questions: v0.8 Compiler Hardening

상태: recommendations awaiting approval

## Q1. Parser recovery boundary

추천: hand-written parser를 유지하고 top-level declaration과 block statement list에서만
synchronization한다. `}`, `func`, `test`, `type`, `import`, `package`처럼 구조가 분명한 token까지
전진하며, 한 source당 최대 32개 frontend diagnostic을 source/span 순으로 출력한다. Parse
error가 하나라도 있으면 semantic/IR 단계로 진행하지 않는다.

이 선택은 cascade를 제한하면서 실제 편집 중 여러 문법 오류를 한 번에 보여준다. JSON은
기존 `mallang.diagnostic.v1` record를 여러 줄 출력하므로 schema 변경이 없다.

## Q2. Panic and invariant policy

추천: user source나 malformed project/IR로 도달 가능한 `panic!`, unchecked indexing과
production `unwrap`을 해당 compiler stage의 `Result<..., Diagnostic>`으로 바꾼다. 내부적으로
증명된 invariant의 assertion은 unit test/debug build에 남기되, blanket `catch_unwind`를 정상
error handling으로 사용하지 않는다.

예상하지 못한 panic은 compiler bug다. 이를 일반 syntax error처럼 숨기면 crash corpus가
고쳐졌다는 근거가 사라지므로 process-wide suppression은 채택하지 않는다.

## Q3. Property and fuzz strategy

추천: stable toolchain에서 재현 가능한 deterministic property generator와 mutation corpus를
canonical CI gate로 둔다. Lexer는 arbitrary UTF-8/no-hang, parser는 token mutation/no-panic,
type/ownership은 known-invalid transformation/rejection property를 검증한다. 실패 input은
최소화해 checked-in crash corpus로 승격한다.

`cargo-fuzz`/nightly와 장시간 random fuzz는 초기 v0.8 필수 dependency로 두지 않는다. 짧은
property gate에서 찾지 못하는 결함의 evidence가 생길 때 별도 scheduled/manual gate로 추가한다.

## Q4. Performance regression policy

추천: representative standalone, multi-project dependency와 standard-library CLI를 대상으로
compile wall time, generated C bytes, native binary bytes와 runtime median을 machine-readable
baseline으로 기록한다. P165 첫 CI 표본은 관찰 전용으로 두고 platform별 분산을 확인한 뒤
절대값과 상대값을 함께 가진 threshold를 별도 승인한다.

측정 전 임의의 비율로 CI를 차단하면 runner noise를 compiler regression으로 오판할 수 있다.
먼저 baseline/variance를 저장하고 threshold를 두 번째 gate에서 결정한다.

## Q5. Reproducibility contract

추천: 같은 compiler/version/input/option에서 generated C와 release archive의 byte identity를
v0.8 contract로 둔다. Native executable은 같은 host에서도 linker/toolchain metadata가 달라질
수 있으므로 실행 behavior와 size를 검증하되 cross-host/cross-toolchain byte identity는
보장하지 않는다.

## Q6. Parser library and LSP scope

추천: P162 결과에서 recovery 복잡도, diagnostic 품질 또는 유지보수 비용의 측정된 문제가
없으면 parser library로 migration하지 않는다. Full LSP도 v0.8 blocker에서 제외한다. P158의
JSONL schema와 P162 multiple diagnostics를 editor foundation으로 유지하고, incremental document
overlay/cancellation 필요성이 구체화될 때 별도 milestone로 연다.

## Approval Decision

Q1-Q6는 아직 제안 상태다. 승인 뒤 `spec.md`의 P161-P166을 순서대로 구현하며, Q4의 실제
regression threshold는 baseline 측정 결과를 제시한 뒤 두 번째 decision gate에서 확정한다.
