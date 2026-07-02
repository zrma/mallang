# Mallang Handoff

## 현재 상태

- 언어 이름: Mallang
- 소스 확장자: `.mlg`
- CLI: `mlg`
- 현재 구현: token model, hand-written lexer, lexer dump CLI
- 아직 없음: parser, AST, type checker, ownership checker, C backend, native build command

## 빠른 시작

```sh
scripts/check.sh
cargo run --bin mlg -- examples/hello.mlg
```

## 주요 문서

- `SPEC.md`: 언어 v0 설계 초안
- `ROADMAP.md`: compiler milestone
- `docs/ROADMAP.md`: agent가 다음 작업을 고르는 운영용 roadmap
- `docs/REPO_MANIFEST.yaml`: 검증 명령과 entrypoint 선언
- `docs/ESCALATION_POLICY.md`: 사용자 호출 조건

## 다음 구현 후보

1. AST 타입 정의
2. recursive descent parser로 function/block/statement 파싱
3. Pratt parser로 expression 파싱
4. parser test fixture 추가
5. `mlg check` subcommand 뼈대 추가
