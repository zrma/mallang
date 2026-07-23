# Spec: first-native-build

Status: complete; historical milestone record

## 목표

- README의 첫 마일스톤을 닫는다: Mallang source를 native binary로 빌드하고,
  첫 target program이 `30`을 출력하게 한다.

## 범위

- `int` 기반 함수 선언/호출 파싱.
- local binding `name := expr` 파싱.
- `return expr` 파싱.
- 산술 expression Pratt parser.
- built-in `print(expr)` statement를 C `printf`로 lower.
- `mlg build <source> -o <binary>` CLI 추가.
- C backend는 첫 milestone subset만 지원한다.
- ownership/borrow checker, `string` concat, `if` expression, `match`는 이번 범위에서
  구현하지 않고 planned 상태로 유지한다.

## C-체크리스트

| ID | 상태 | Verify command | 작업 항목 |
| --- | --- | --- | --- |
| C1 | done | `scripts/check.sh` | parser/build subset 구현 |
| C2 | done | `target/mallang/first` | 첫 target binary가 `30` 출력 |

## 완료 기준

- C-체크리스트가 완료되고 검증 명령이 통과한다.

## 검증 증거

- `scripts/check.sh`
- `cargo run --bin mlg -- build examples/first.mlg -o target/mallang/first`
- `target/mallang/first` -> `30`
