# P165: Performance and Reproducibility Baseline

상태: complete (2026-07-16)

## Measurement contract

`scripts/measure-v08-baseline.py`는 release profile의 `mlg`로 다음 네 repository-owned
입력을 측정한다.

| Case | Input | Recorded metrics |
| --- | --- | --- |
| minimal standalone | `examples/first.mlg` | check/build median, generated C/native bytes |
| cleanup-heavy standalone | `examples/full-expression-cleanup.mlg` | check/build/runtime median, sizes, output |
| local dependency project | `examples/projects/local-deps/app` | check/build median, generated C/native bytes |
| standard-library CLI | `examples/projects/textstats` | check/build/runtime median, sizes, fixture output |

각 wall-time은 one warmup 뒤 7회 측정의 median이다. Runtime output은 UTF-8 text와
SHA-256을 함께 기록하고, generated C는 bytes와 SHA-256을 기록한다. 측정 환경은 OS와
architecture만 남기며 hostname, username, absolute path와 내부 환경 정보는 기록하지
않는다.

현재 관찰값은 `docs/baselines/v0.8-performance.json`이 소유한다. 이 baseline은
`observational`이며 regression threshold는 `null`이다. Supported-platform CI variance를
검토하기 전에는 wall-time이나 size 변화만으로 CI를 실패시키지 않는다.

재측정 명령:

```sh
cargo build --release --bin mlg
scripts/measure-v08-baseline.py \
  --compiler target/release/mlg \
  --output docs/baselines/v0.8-performance.json
```

## Reproducibility contract

`scripts/check-v08-reproducibility.sh`는 다음 범위를 하나의 gate로 검증한다.

- baseline schema, representative case set과 observational policy
- 동일 compiler, input, options와 host에서 네 case의 generated C byte identity
- 기존 `scripts/check-release-artifacts.sh`가 보장하는 repeated release archive byte identity

Native executable byte identity는 host C compiler, linker와 toolchain metadata 영향을 받으므로
계약에서 제외한다. Native binary bytes는 성능 관찰값일 뿐 reproducibility 판정값이 아니다.

## Initial observation

2026-07-16 macOS/aarch64 관찰에서 check median은 약 4-6 ms, native build median은
약 69-85 ms였다. Generated C는 약 3.4-61.4 KB, native executable은 약 33.6-55.1 KB
범위였다. Cleanup-heavy와 textstats fixture runtime median은 약 3.5-4.0 ms였다.

이 값은 hardware-independent 성능 약속이 아니다. P166 acceptance에서는 수치 자체가 아니라
schema 유효성, output stability와 byte-identity gate 통과를 증거로 사용한다.

## Completion evidence

- [x] 네 representative case를 release compiler로 반복 측정
- [x] machine-readable observational baseline과 environment schema 고정
- [x] runtime output text/hash와 generated C size/hash 기록
- [x] generated C repeated-build byte identity gate 추가
- [x] release archive repeated-build byte identity gate 연결
- [x] native executable identity 제외 범위 문서화
- [x] canonical check와 repository manifest 연결
