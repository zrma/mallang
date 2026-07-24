# Spec: cli-version-smoke

Status: complete; historical milestone record

## 목표

- v0 PoC의 user-facing CLI가 자기 버전을 출력할 수 있게 한다.
- `Cargo.toml` package version과 `mlg --version` 출력이 같은지 full smoke에서
  검증한다.

## 범위

- `mlg --version`과 `mlg -V`를 추가한다.
- `usage` output에 version flag를 표시한다.
- `scripts/check.sh`에 CLI version smoke를 추가한다.
- README/SPEC/HANDOFF/ROADMAP에 version command를 기록한다.

## 제외

- SemVer policy 설계.
- Release artifact packaging.
- `mlg version` subcommand 추가.

## C-체크리스트

| ID | 상태 | Verify command | 작업 항목 |
| --- | --- | --- | --- |
| C1 | done | `cargo run --quiet --bin mlg -- --version` | CLI version output |
| C2 | done | `scripts/check.sh` | full repo smoke includes version check |

## 완료 기준

- C-체크리스트가 완료되고 검증 명령이 통과한다.
