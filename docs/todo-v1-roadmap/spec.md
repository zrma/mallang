# Spec: v1-roadmap

## 목표

- Mallang `v0.2.0`부터 `v1.0.0`까지 아홉 개 마일스톤을 durable roadmap으로
  고정한다.
- 현재 구현과 장기 계획이 섞이지 않도록 planned/implemented 경계를 명확히 한다.

## 범위

- `docs/V1_ROADMAP.md`에 각 마일스톤의 목표, 범위, 완료 조건, 제외 항목을
  기록한다.
- v1의 안정성 의미와 공통 검증 규칙을 기록한다.
- `AGENTS.md`, `README.md`, `SPEC.md`, `ROADMAP.md`, `docs/ROADMAP.md`,
  `docs/HANDOFF.md`, `docs/agent-harness.md`에서 장기 roadmap을 찾을 수 있게
  연결한다.

## 제외

- 새 language syntax 구현
- lexer, parser, semantic, IR, backend 변경
- v0.2 세부 문법의 선결정
- version bump, tag, release, push

## 체크리스트

| ID | 상태 | 검증 | 작업 |
| --- | --- | --- | --- |
| C1 | done | milestone heading count | `v0.2.0`부터 `v1.0.0`까지 아홉 개 milestone 기록 |
| C2 | done | roadmap review | 각 milestone에 범위와 완료 조건 기록 |
| C3 | done | repository link scan | 주요 진입 문서에서 `docs/V1_ROADMAP.md` 연결 |
| C4 | done | `scripts/check-agent-harness-interface.sh` | repository harness contract 유지 |

## 완료 기준

- 장기 roadmap이 현재 v0.1 spec과 구분되어 있다.
- 다음 구현 시작점이 `v0.2.0: Projects and Modules`로 명확하다.
- 아직 열려 있는 language design은 decision gate로 표시되어 있다.
