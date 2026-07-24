# Work Packet Artifact Lifecycle

Status: complete; migrated completed work packets and enforced the artifact lifecycle

## 목표

- 진행 중이거나 결정을 기다리는 work packet과 완료된 구현 근거를 경로로
  명확히 구분한다.
- 완료 패킷을 `docs/artifacts/<work-id>/`에 보존하고 다시 `todo` 영역에
  쌓이지 않도록 생성·완료·검증 계약을 고정한다.

## 범위

- `docs/todo-*`의 완료 패킷을 `docs/artifacts/<work-id>/`로 이동한다.
- 저장소의 완료 패킷 참조를 새 경로로 갱신한다.
- `scripts/start-work.sh`, `scripts/archive-work.sh`,
  `scripts/check-todo-state.py`가 동일한 lifecycle을 강제한다.
- 활성, 보류, 결정 대기 패킷은 기존 `docs/todo-<work-id>/` 형식을
  유지한다.
- 구현 근거의 내용을 다시 작성하거나 역사적 결정을 합치지 않는다.

## C-체크리스트

| ID | 상태 | Verify command | 작업 항목 |
| --- | --- | --- | --- |
| C1 | done | `find docs -maxdepth 1 -type d -name 'todo-*'` | `todo`에는 미완료 패킷만 남긴다. |
| C2 | done | `python3 scripts/check-todo-state.py` | todo/artifact 상태와 참조 불변식을 검증한다. |
| C3 | done | `scripts/check-release-helpers.sh` | start/archive helper 계약과 shell 문법을 검증한다. |
| C4 | done | `scripts/check.sh` | canonical 저장소 게이트를 통과한다. |

## 완료 기준

- 완료 패킷이 `docs/todo-*`에 남지 않는다.
- 완료 아티팩트와 미완료 패킷의 work ID가 중복되지 않는다.
- 완료 아티팩트를 가리키는 `docs/todo-<id>` 참조가 남지 않는다.
- 새 work ID가 기존 아티팩트를 덮어쓸 수 없다.
- C-체크리스트가 완료되고 검증 명령이 통과한다.
