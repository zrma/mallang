# Completed Work Artifacts

이 디렉터리는 완료된 work packet의 설계 결정, 구현 체크리스트와 검증
근거를 보존한다. 각 아티팩트는 `docs/artifacts/<work-id>/`에 있으며
`spec.md`의 상태는 `complete`여야 한다.

현재 작업과 보류·결정 대기 패킷은 `docs/TODO_INDEX.md`와
`docs/todo-<work-id>/`가 소유한다. 완료할 때는 상태와 체크리스트를 닫은
뒤 다음 명령으로 아티팩트화한다.

```sh
scripts/archive-work.sh --work-id <work-id>
python3 scripts/check-todo-state.py
```

완료 근거는 제목이나 본문으로 검색한다.

```sh
rg -l '<term>' docs/artifacts
```
