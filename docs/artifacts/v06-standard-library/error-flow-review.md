# Error Flow Review: v0.6 Reference CLI

상태: complete (2026-07-15)

## Measured program

`examples/projects/textstats`는 source package가 둘인 native CLI다.

- `main` package는 `os.args`, `fs.readText`, `fs.writeText`, `io.writeStdout`과
  `io.writeStderr`를 사용한다.
- `stats` package는 UTF-8 byte/scalar count, newline split과 `Map[int,int]`를 사용해
  line-length histogram을 만들고 summary text를 반환한다.
- CLI는 `textstats <input> [output]` 형태이며 output이 없으면 stdout에 쓰고, expected
  failure는 stderr와 non-zero exit로 변환한다.

Acceptance는 stdout/output-file success, usage exit 2, missing file, invalid UTF-8 input,
write failure, strict C, zero live allocation과 ASan/UBSan을 검증한다.

## Boilerplate measurement

현재 source 기준 측정값은 다음과 같다.

| 항목 | 값 |
| --- | ---: |
| `main.mlg` | 72 lines |
| `stats.mlg` | 40 lines |
| `Result` call sites in `main` | 5 |
| exhaustive `Result` matches | 5 |
| `Ok`/`Err` arms | 10 |
| deepest nested `Result` handling | 3 levels |

`ExitWithError`가 operation context, `errors.Kind`, stderr와 exit 1 변환을 한 곳으로
모으므로 네 error branch의 출력 코드는 중복되지 않는다. 반면 성공 값을 다음 단계로
전달하려면 `os.args -> fs.readText -> write`의 match nesting은 남는다. 순수 transformation인
`stats` package에는 `Result` match가 없다.

## `?` decision

v0.6에는 `?`를 추가하지 않는다.

- Rust-style postfix `?`는 nullable optional chaining이 아니라 `Result`/`Option`의 조기
  반환 연산이어야 한다.
- Reference CLI의 error branch는 단순 전달이 아니라 operation context, stderr와 process
  exit code로 변환한다. 현재 `func main()`은 `unit`이므로 `?`만 추가해도 이 경계는
  사라지지 않는다.
- 안전한 조기 반환에는 return type compatibility, local cleanup 순서, `Result`와 `Option`
  적용 범위, error conversion, `main` result/exit policy를 함께 결정해야 한다. Parser sugar
  하나로 취급할 수 없다.
- 이번 한 프로그램은 reusable `Result`-returning library function에서 반복 전달이 얼마나
  발생하는지 보여주지 않는다.

따라서 v0.8 hardening 전까지 syntax work를 예약하지 않는다. 추가 실제 프로그램에서
동일 error를 그대로 전달하는 반복이 확인되면 v0.9 language freeze 전에 별도 decision
gate를 열고, 그때 cleanup IR과 process boundary를 함께 설계한다.
