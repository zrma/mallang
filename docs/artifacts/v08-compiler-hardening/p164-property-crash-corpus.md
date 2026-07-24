# P164: Property and Crash-corpus Testing

상태: complete (2026-07-16); P165-P166 complete, released as v0.8.0

## Deterministic property contract

Canonical property tests use only stable Rust and fixed generators. They do not depend on wall
clock time, OS entropy, nightly Rust or an external fuzzing service.

| Property | Input space | Required result |
| --- | --- | --- |
| lexer UTF-8 | 256 fixed LCG seeds, lengths 0..64, syntax-heavy ASCII plus generated Unicode scalar values | success tokens end in EOF; every token/error span is ordered, in bounds and on UTF-8 boundaries |
| parser token mutation | four valid programs; every token is deleted, duplicated or replaced by five representative token kinds | parsing returns a program or 1..32 stable ordered diagnostics without a panic |
| type/ownership rejection | five valid programs transformed at one exact marker | the original checks successfully and the transformed program returns the expected semantic message class |

The parser property deliberately mutates public token input rather than source text only. This
exercises the P163 EOF normalization and recovery cursor contract independently of the lexer.

## Checked-in crash corpus

`tests/fixtures/hardening/crash-corpus/` contains six minimized `.mlg` programs covering frontend,
package, link, semantic and ownership failures. The integration test declares the expected compiler
stage and message class for each file and fails if an undeclared `.mlg` file appears in the corpus.

Newly discovered compiler crashes or unstable diagnostics must be reduced to the smallest useful
source, added to this directory, and registered in `tests/hardening_properties.rs`. A corpus case is
complete only when it returns a stage-owned diagnostic; process-wide panic suppression is not an
acceptable expectation.

## Canonical gate

```sh
cargo test --test hardening_properties
```

This command is part of `docs/REPO_MANIFEST.yaml` and therefore runs inside the existing
`cargo test --workspace` repository gate. Current evidence is four integration tests passed: lexer
UTF-8, parser token mutation, type/ownership invalid transformation and checked-in crash corpus.

Nightly `cargo-fuzz` remains deferred. P165 owns representative compile/runtime measurements and
same-input generated-output reproducibility; P166 connects the corpus to release-binary acceptance.
