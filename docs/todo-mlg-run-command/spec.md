# Spec: mlg-run-command

## 목표

- SPEC에 이미 선언된 `mlg run` command를 CLI에 구현한다.
- `mlg run <source-file>`은 Mallang source를 native binary로 build한 뒤 바로
  실행한다.
- build artifact path 같은 compiler 출력은 숨기고, 실행된 Mallang program의
  stdout/stderr를 그대로 사용자에게 전달한다.

## 범위

- CLI:
  - usage에 `run` subcommand를 추가한다.
  - `build`와 `run`이 parse/check/generate C/clang compile 경로를 공유한다.
  - `run`은 `target/mallang/run/<source-stem>`에 binary를 만든 뒤 실행한다.
  - compiled program이 non-zero로 종료하면 `mlg run`도 실패한다.
- Smoke:
  - `examples/range-index.mlg`를 `mlg run`으로 실행해 native output을 검증한다.
- 이번 slice에서는 command-line argument passing to Mallang `main`,
  incremental rebuild cache, temporary artifact cleanup은 다루지 않는다.

## C-체크리스트

| ID | 상태 | Verify command | 작업 항목 |
| --- | --- | --- | --- |
| C1 | done | `scripts/check.sh` | `mlg run` CLI and shared compile path |
| C2 | done | `scripts/check.sh` | native run smoke and non-zero failure propagation |
| C3 | done | `scripts/check.sh` | docs/spec/roadmap/handoff 갱신 |

## 완료 기준

- C-체크리스트가 완료되고 검증 명령이 통과한다.
