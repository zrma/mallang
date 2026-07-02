# Mallang Agent Guide

## 저장소 의도

- Mallang은 Go-like syntax, Rust-like safety, functional value style을 실험하는 네이티브 언어 PoC다.
- 코드와 테스트가 source of truth다. 문서는 navigation, decision record, safety boundary, acceptance criteria를 소유한다.
- 문서가 구현을 재서술하고 있으면 관련 코드/스크립트/테스트 포인터로 줄인다.

## 자율 실행 원칙

- 에이전트는 사용자에게 세부 지시를 요구하지 않고, 현재 문서/스크립트/테스트 기준으로 목표 달성까지 진행한다.
- 비사소한 작업은 `docs/todo-<work-id>/`에 `spec.md`와 `open-questions.md`를 먼저 고정한다.
- 미결 질문이 없고 검증 경로가 명확하면 구현, 검증, 문서 갱신, `jj` change 정리, push까지 같은 작업 단위에서 닫는다.
- 사용자 호출은 `docs/ESCALATION_POLICY.md`의 조건에 해당하거나 제품/언어 설계 결정이 여러 방향으로 갈리는 경우로 제한한다.

## 작업 단위

- 한 change는 하나의 명확한 목적을 가진다: 예를 들어 lexer 확장, parser milestone, ownership rule 문서화, release gate 보강.
- 여러 compiler phase를 동시에 건드릴 때는 frontend, semantic analysis, ownership, backend, docs/test로 boundary를 나눈다.
- `jj split`, `jj squash`, `jj rebase`, 원격 bookmark 이동처럼 히스토리에 영향이 있는 작업은 사용자 승인 없이 실행하지 않는다.

## 검증

- 기본 검증은 `scripts/check.sh`다.
- lexer/parser 변경은 단위 테스트와 예제 입력 검증을 추가한다.
- 언어 스펙 변경은 `SPEC.md`와 `ROADMAP.md`를 함께 확인하고, 코드가 아직 따라오지 않았으면 명시적으로 "planned" 상태를 남긴다.
- 최종 보고에는 `jj status`, 실행한 검증, push 여부를 포함한다.

## VCS

- 로컬 VCS 작업은 `git`보다 `jj`를 우선 사용한다.
- Codex가 change description을 작성하거나 수정할 때는 `~/.codex/skills/vcs-jj/scripts/describe_with_attribution.sh`를 사용한다.
- 메시지 형식은 `<type>: <summary>`이며 scope 괄호는 사용하지 않는다.
- 기본 bookmark는 `main`이다.

## 무컨텍스트 다음 순서

- 새 작업은 `docs/HANDOFF.md` -> `docs/ROADMAP.md` -> `docs/REPO_MANIFEST.yaml` -> `scripts/check.sh` 순서로 현재 상태를 확인한다.
- 활성 `docs/todo-*`가 있으면 그 spec을 우선한다.
