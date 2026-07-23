# Self-hosted Check Performance

Status: complete; reduced the large-project self-hosted check path below the local target

## Goal

Reduce the release `mlg 1.2.0` default self-hosted `check` latency for the
repository-owned compiler project while preserving small-project latency and
all Stage0/self-hosted correctness gates.

The local reference input contains 17 Mallang source files, about 23,243 lines,
and about 1.1 MB of source. One warmup followed by five process-level samples
produced these initial medians:

| Compiler | `mlg check bootstrap/compiler` |
| --- | ---: |
| default self-hosted | 2,795 ms |
| explicit Stage0 | 160 ms |

These are host-local observations, not portable performance promises.

The reproducible measurement command is:

```sh
scripts/measure-self-hosted-check-performance.py \
  --compiler target/release/mlg \
  --max-self-ms 1000 \
  --output <local-output>
```

The script emits source inventory, Stage0/self medians, min/max values, and raw
samples as `mallang.self-hosted-check-performance.v1` JSON. The output remains a
local observation and is not checked into the repository.

## Profile Evidence

A two-second macOS `sample` capture of direct optimized `mlgc check-project`
execution collected 1,633 main-thread samples. Of those, 1,310 samples ended in
`mallang_utf8_scalar_count_bytes`, primarily below `mallang_string_equal`.

The self-hosted semantic and linker passes compare identifier and type-shape
strings frequently. The previous equality helper performed a complete UTF-8
scan of both values even though Mallang strings had already crossed a validating
or validity-preserving construction or external-input boundary. The repeated
scans dominated large-project check time.

## Optimization Boundary

- Keep full UTF-8 validation at external input, owned-string construction, and
  operations that interpret Unicode scalar boundaries.
- Keep storage kind, non-null data, and length guards at byte-equality use sites.
- Compare already-valid strings by length and bytes without rescanning their
  complete UTF-8 contents.
- Apply the same runtime helper change to the Rust Stage0 and Mallang
  self-hosted C backends so fixed-point and differential output remain aligned.
- Do not add incremental compilation or persistent caches in this slice.

Invalid internal native layouts remain outside valid Mallang source behavior.
The explicit full validator and invalid-UTF-8 ingress tests continue to reject
malformed data.

## Result

After changing both generated equality helpers to validate layout and compare
bytes, one warmup followed by seven process-level samples produced these local
medians:

| Compiler | Before | After |
| --- | ---: | ---: |
| default self-hosted | 2,795 ms | 762 ms |
| explicit Stage0 | 160 ms | 160 ms |

The default self-hosted check is about 3.7 times faster, a 72.8% latency
reduction. Representative post-change check medians were 6.5 ms for the minimal
fixture, 7.8 ms for the cleanup-heavy fixture, 17.9 ms for the local-dependency
project, and 17.7 ms for the textstats project. These remain host-local
observations rather than supported-platform guarantees.

The B3 backend gate passed 14 positive fixtures and nine runtime-rejection
cases. The B4 fixed-point gate passed full conformance, 15 backend fixtures,
eight backend projects, 21 native pairs, and sanitizer execution.

## Acceptance

- [x] Generated equality helpers use layout validation for both operands.
- [x] Invalid storage, overflow, and invalid UTF-8 boundary tests still reject.
- [x] Stage0 and self-hosted generated C remain equivalent.
- [x] Representative small-project `check` medians do not materially regress.
- [x] Compiler-project default self-hosted `check` reaches a local median below
  1,000 ms.
- [x] Focused, fixed-point, sanitizer, and canonical repository gates pass.
- [x] Final before/after observations and measurement commands are recorded.
