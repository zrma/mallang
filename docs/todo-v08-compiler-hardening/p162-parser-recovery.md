# P162: Parser Recovery and Multiple Diagnostics

상태: complete (2026-07-16); P163 next

## Slice A contract

Top-level parser recovery와 deterministic multi-source aggregation을 다음 호환 경계로
추가했다.

- 기존 `parse`, `parse_with_source`, `parse_sources`와 compiler entrypoint는 첫 오류를
  반환하는 convenience API로 유지한다.
- parser의 `parse_with_diagnostics`/`parse_with_source_diagnostics`, frontend의
  `parse_sources_with_diagnostics`, compiler의 `*_with_diagnostics` entrypoint는 ordered
  error vector를 반환한다.
- CLI `parse`, `check`, `ir`, `build`, `run`, `test`는 multi-diagnostic compiler/frontend
  경로를 사용한다.
- source 하나에서 최대 32개 parse error를 수집한다. Lexical error는 계속 첫 오류에서
  중단한다.
- parse error가 하나라도 있으면 partial `Program`을 반환하지 않고 package, semantic, IR,
  backend 단계에 진입하지 않는다.

## Recovery boundary

오류가 발생하면 parser cursor를 현재 top-level declaration 시작으로 되돌린 뒤 최소 한
token을 소비한다. 이후 delimiter depth를 추적하면서 다음 `pub`, `type`, named `func`,
contextual `test`, `package`, `import` boundary로 이동한다. Malformed function/type body의
closing brace를 만나면 해당 declaration에서 닫히지 않은 parenthesis/bracket depth를
폐기해 다음 top-level declaration을 찾는다.

Recovery target에서 receiver method의 `func (` 형태는 의도적으로 제외했다. Inner
function literal과 receiver method를 구분할 newline token이 없으므로 보수적인 named-function
boundary만 사용한다.

## Slice B contract

Block parser는 source 전체 diagnostic accumulator를 공유하고 다음 경계만 사용한다.

- depth 0의 explicit `;`를 소비한 뒤 다음 statement로 이동
- brace depth 0의 `}`를 unmatched parenthesis/bracket보다 우선해 현재 block 종료로 처리
- `return`, `for`, `break`, `continue`, `mut` keyword에서만 statement parsing 재개
- named `func` 등 unambiguous top-level declaration이 block 안에서 보이면 missing `}`를
  보고하고 top-level recovery에 위임
- EOF 또는 identifier-led ambiguity에 안전한 경계가 없으면 나머지 block을 폐기

`if`와 `match`는 expression 위치에도 올 수 있고 identifier-led expression은 newline 없이
다음 statement와 구분할 수 없으므로 recovery starter로 사용하지 않는다.

## Evidence

- parser unit test: 두 top-level syntax error의 span order와 기존 첫 오류 API의 동일성
- parser cap test: source 하나의 40개 malformed declaration에서 정확히 32개 diagnostic
- frontend unit test: 두 source의 오류가 caller-provided source order로 집계됨
- compiler unit test: frontend 오류만 반환되고 뒤의 unknown symbol semantic 오류는 없음
- block unit tests: 한 block의 복수 오류, source-wide 32-error cap, unmatched parenthesis에서
  현재 block `}` 우선 처리
- nested regression: inner function literal 유지, unclosed block의 named-function handoff,
  receiver method의 ambiguous target 제외
- CLI fixture: 두 project source의 top-level/block human/JSON frontend diagnostic 4개가 같은
  순서로 렌더링되고 non-zero로 종료함
- `cargo test --all-targets`: 571 tests passed
- `cargo clippy --all-targets -- -D warnings`: passed
- `scripts/check-diagnostics.sh target/debug/mlg`: passed

## Slice C contract

- Exact `(message, span)` duplicate만 제거하고, 같은 span의 서로 다른 message는 보존한다.
- Source 내부 diagnostic은 span start/end의 stable order로 반환하고 frontend는 caller-provided
  source order를 유지한다.
- 40개 malformed statement acceptance는 line 2부터 33까지의 concrete error 32개만
  반환한다. Approved maximum에 별도 33번째 truncation summary는 추가하지 않는다.
- `parse`, `check`, `ir`, `build`, `run`, `test`는 모두 non-zero, empty stdout와 동일한
  human/JSON record order를 보장한다.
- Lexical error는 계속 첫 record에서 중단하고 기존 single-error API는 multi report의 첫
  error와 동일하다.

Canonical acceptance는 `scripts/check-parser-recovery.sh target/debug/mlg`가 소유하며
`scripts/check.sh`에서 항상 실행한다.
