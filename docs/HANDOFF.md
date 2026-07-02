# Mallang Handoff

## 현재 상태

- 언어 이름: Mallang
- 소스 확장자: `.mlg`
- CLI: `mlg`
- 현재 구현: token model, hand-written lexer, AST, parser, semantic checker, ownership-lite move/borrow checks, same-call borrow conflict checks, first native subset C backend, `mlg check`, `mlg build`
- 아직 없음: statement-spanning borrow lifetimes, typed IR, full C backend, `if`/`match`

## 빠른 시작

```sh
scripts/check.sh
cargo run --bin mlg -- check examples/first.mlg
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

1. typed IR 도입 여부 결정
2. `if` expression parser/type checker 추가
3. `Option[T]` / `Result[T, E]` surface 설계
4. backend를 checked representation 기반으로 더 강하게 분리
5. statement-spanning borrow lifetimes가 필요한 syntax가 생기는지 점검
