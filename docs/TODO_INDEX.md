# Work Packet Index

Status: canonical

`docs/todo-*`는 활성, 보류, 결정 대기 work packet만 보존한다. 구현이
완료된 패킷은 `docs/artifacts/<work-id>/`로 이동하며, 구현 당시의 결정과
검증 근거는 그 경로에서 유지한다.

## Active

현재 활성 패킷 없음.

## Decision Required

현재 사용자 결정을 기다리는 패킷 없음.

## Deferred

- [Naming conventions](todo-naming-conventions/spec.md): 1.x `mlg lint`는
  완료됐고 reference-aware rename과 hard-error는 2.0 경계로 보류한다.
- [Explicit error-flow ergonomics](todo-explicit-error-flow-ergonomics/spec.md):
  `?` 없이 expression `match`의 명시적 returning arm으로 중첩을 줄이는
  compatible 1.x 설계를 기록했다.
- [Structural interface typing](todo-structural-interface-typing/spec.md):
  named data는 nominal로 유지하고 method-set 만족 관계만 structural로
  검토한다. 첫 후보는 static generic constraint로 제한하며 runtime interface와
  dynamic dispatch는 별도 decision gate로 둔다.

## Completed Programs

- v0.1-v1.0 언어·컴파일러·배포 마일스톤
- v1.1 streaming text I/O
- v1.2 self-hosting B0-B5와 기본 컴파일러 전환
- self-hosted compiler project `check` 성능 개선
- 세부 ownership, parser, backend, runtime 결정과 회귀 작업

완료 기록은 다음 명령으로 찾는다.

```sh
rg -l '^Status: complete' docs/artifacts/*/spec.md
```

## State Contract

- `docs/todo-*`의 허용 상태는 `active`, `deferred`,
  `decision-required`다.
- `docs/artifacts/<work-id>/spec.md`의 상태는 `complete`여야 한다.
- 상태는 각 `spec.md` 첫 8줄 안에 `Status: <state>; <detail>` 형태로 둔다.
- todo 패킷은 `open-questions.md`를 포함하고 이 인덱스에 링크한다.
- complete 아티팩트는 미완료 checkbox나 C-checklist 행을 남기지 않는다.
- 새 패킷은 `scripts/start-work.sh --work-id <id>`로 만든다.
- 완료 패킷은 `scripts/archive-work.sh --work-id <id>`로 이동한다.
- 정합성 검사는 `python3 scripts/check-todo-state.py`가 담당한다.
