# Mallang Handoff

## 현재 상태

- 언어 이름: Mallang
- 소스 확장자: `.mlg`
- CLI: `mlg`
- 현재 구현: token model, hand-written lexer, AST, parser, first native subset C backend, `mlg build`
- 아직 없음: full type checker, ownership checker, typed IR, full C backend, `if`/`match`

## 빠른 시작

```sh
scripts/check.sh
cargo run --bin mlg -- build examples/first.mlg -o target/mallang/first
target/mallang/first
```

## 주요 문서

- `SPEC.md`: 언어 v0 설계 초안
- `ROADMAP.md`: compiler milestone
- `docs/ROADMAP.md`: agent가 다음 작업을 고르는 운영용 roadmap
- `docs/REPO_MANIFEST.yaml`: 검증 명령과 entrypoint 선언
- `docs/ESCALATION_POLICY.md`: 사용자 호출 조건

## 다음 구현 후보

1. `mlg check` subcommand 뼈대 추가
2. immutable binding reassignment reject
3. unresolved identifier와 `nil` diagnostics 강화
4. ownership-lite move checker 시작
5. typed IR 도입 여부 결정
