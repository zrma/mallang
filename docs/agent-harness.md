# Agent Harness

## Interface

- Structure ID: `agent-harness-v1`.
- Baseline ID: `openai-gpt-5.6-2026-07-10`.
- Convergence stage: `canonical`.
- Target stage: `canonical`.
- Canonical check: `scripts/check-agent-harness-interface.sh`.

`AGENTS.md`가 공통 GPT-5.6 계약을 소유하고, 이 문서는 Mallang compiler overlay와 기존 handoff 문서로 가는 canonical 진입점이다.

## Project Objective

Go-like syntax, Rust-like safety, functional value style을 탐색하는 native language PoC를 작고 검증 가능한 compiler milestone로 발전시킨다.

## Source Of Truth

- 언어 동작: compiler source, tests, examples, generated C와 native smoke.
- 현재 상태와 읽기 순서: `docs/HANDOFF.md`.
- 방향과 planned/implemented 경계: `SPEC.md`, `docs/ROADMAP.md`.
- 현재 작업 계약: 활성 `docs/todo-*/spec.md`와 `open-questions.md`.

## Autonomy And Permissions

- 목표와 검증 경로가 명확한 로컬·가역 작업은 추가 승인 없이 구현, 검증, 문서화, local change 정리까지 진행한다.
- 외부 write, secret, 비용, 파괴적 작업, 제품 방향 변경, 승인되지 않은 원격 변경은 에스컬레이션한다.
- 언어 설계가 여러 호환 불가능한 방향으로 갈리고 기존 spec/roadmap이 결정하지 않으면 사용자 판단을 요청한다.

## Execution Loop

1. `jj status`, `docs/HANDOFF.md`, 활성 todo를 확인한다.
2. lexer, parser, semantic, ownership, IR/backend, runtime 중 변경 경계를 고정한다.
3. 비사소한 작업은 todo acceptance와 open question을 먼저 닫는다.
4. 실패 테스트 또는 예제 입력을 먼저 고정하고 최소 범위로 구현한다.
5. focused test와 generated/native smoke를 실행한다.
6. 구현과 spec/roadmap 상태가 달라지면 같은 change에서 문서를 갱신한다.
7. 하나의 compiler 목적을 가진 `jj` change로 닫는다.

## Verification And Evidence

- Harness interface: `scripts/check-agent-harness-interface.sh`.
- 기본 full gate: `scripts/check.sh`.
- frontend 변경: lexer/parser/semantic targeted tests와 example diagnostics.
- backend/runtime 변경: generated C warning/sanitizer 검사와 native smoke.
- 최종 증거에는 acceptance별 test, planned/implemented 문서 상태, local/remote bookmark, CI를 포함한다.

## Escalation

`docs/ESCALATION_POLICY.md`를 기준으로, 언어 surface 결정, 호환성 tradeoff, 외부 의존/비용, 파괴적 작업, published history rewrite, 승인되지 않은 push가 필요한 경우에만 사용자에게 최소 판단을 요청한다.

## VCS And Publish

- 로컬 VCS는 `jj`를 사용하고 change description은 `<type>: <summary>`와 Codex trailer 규칙을 따른다.
- compiler phase가 여러 개면 논리 경계별 change로 나누고 기존 사용자 변경을 보존한다.
- 검증된 마일스톤만 로컬 `main`으로 전진시킨다.
- push 권한이 주어진 경우 원격 freshness, commit, CI, release smoke를 해당 범위에 맞게 확인한다.

## Harness Evaluation And Improvement

대표 compiler task에서 완료성, diagnostics/evidence 품질, 회귀율, 지연, 비용을 평가한다. 반복 실패는 작은 regression test, todo acceptance, gate script 또는 concise handoff 규칙으로 고정한다.

## Convergence

- `bridge`: 이 문서가 공통 인터페이스를 제공하고 기존 상세 문서를 연결한다.
- `normalized`: 중복된 autonomy, execution, verification, escalation, VCS 정책을 이 문서의 동일 섹션으로 이동한다.
- `canonical`: 프로젝트 목적, source, command, domain invariant는 같은 섹션 계약 안의 local content로 유지하고 공통 baseline, 제목 순서, 검사 골격은 동일하게 잠근다.
- 단계 전환은 현재 저장소의 Structure ID, 섹션 순서, canonical check 결과로 검증하며 다른 저장소의 이름·개수·로컬 경로·공개 여부를 전제하지 않는다.

## Project Overlay

- 코드와 테스트가 실제 언어 동작의 기준이며 planned 기능을 implemented로 표시하지 않는다.
- ownership/safety 규칙은 positive와 rejection regression을 함께 요구한다.
- generated C뿐 아니라 native 실행 결과까지 확인한다.

## Related Documents

- Navigation and status: `docs/HANDOFF.md`.
- Language direction: `SPEC.md`, `docs/ROADMAP.md`.
- Escalation: `docs/ESCALATION_POLICY.md`.
- Repository command map: `docs/REPO_MANIFEST.yaml`.
