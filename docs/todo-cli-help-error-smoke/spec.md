# Spec: cli-help-error-smoke

## 목표

- v0 PoC CLI의 help/error stream behavior를 smoke로 고정한다.
- 성공 help는 stdout으로, error usage/diagnostic은 stderr로 분리한다.

## 범위

- `mlg --help` / `mlg -h`가 usage를 stdout으로 출력한다.
- source argument 없이 실행한 경우 usage를 stderr로 출력하고 non-zero로 종료한다.
- unknown subcommand는 stderr diagnostic과 non-zero exit를 유지한다.
- `scripts/check.sh`에 help/no-args/unknown-command smoke를 추가한다.

## 제외

- Subcommand별 help.
- CLI argument parser dependency 도입.
- Error code taxonomy 설계.

## C-체크리스트

| ID | 상태 | Verify command | 작업 항목 |
| --- | --- | --- | --- |
| C1 | done | `cargo run --quiet --bin mlg -- --help` | help stdout output |
| C2 | done | `scripts/check.sh` | full repo smoke includes CLI stream checks |

## 완료 기준

- C-체크리스트가 완료되고 검증 명령이 통과한다.
