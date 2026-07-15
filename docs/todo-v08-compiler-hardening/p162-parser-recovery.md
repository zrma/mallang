# P162: Parser Recovery and Multiple Diagnostics

상태: in progress; Slice A complete (2026-07-16), Slice B next

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

Recovery target에서 receiver method의 `func (` 형태는 의도적으로 제외했다. Slice A는
inner function literal을 top-level method로 오인하지 않는 보수적인 named-function
boundary만 소유한다. Block 내부 statement recovery와 receiver-method 재개는 Slice B에서
nested delimiter regression과 함께 다룬다.

## Evidence

- parser unit test: 두 top-level syntax error의 span order와 기존 첫 오류 API의 동일성
- parser cap test: source 하나의 40개 malformed declaration에서 정확히 32개 diagnostic
- frontend unit test: 두 source의 오류가 caller-provided source order로 집계됨
- compiler unit test: frontend 오류만 반환되고 뒤의 unknown symbol semantic 오류는 없음
- CLI fixture: 두 project source의 human/JSON frontend diagnostic이 같은 두 record를 같은
  순서로 렌더링하고 non-zero로 종료함
- `cargo test --all-targets`: 563 tests passed
- `cargo clippy --all-targets -- -D warnings`: passed
- `scripts/check-diagnostics.sh target/debug/mlg`: passed

## Remaining P162 work

### Slice B: block statement recovery

- `;`, current block `}`, unambiguous statement keyword를 사용한 delimiter-aware recovery
- identifier-led ambiguity에서는 block을 포기하고 top-level recovery로 위임
- nested block/function literal이 top-level declaration으로 오인되지 않는 regression

### Slice C: cap and compatibility acceptance

- duplicate suppression과 32-error truncation contract 고정
- full command surface human/JSON order 및 non-zero compatibility acceptance
- lexical fail-fast와 single-error convenience API를 release contract에 반영
