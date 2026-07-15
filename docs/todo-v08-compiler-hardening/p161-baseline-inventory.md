# P161: Hardening Baseline Inventory

상태: complete (2026-07-16); P162-P165 complete, P166 next

## Frontend recovery baseline

Current frontend error flow is deliberately fail-fast at every layer:

- `Lexer::lex_all` returns the first `LexError`; it does not emit a token stream after an invalid
  character, escape, string, or reserved internal identifier.
- `Parser::parse_program` and every declaration/statement parser return one `ParseError` through
  `?`. There is no cursor synchronization or partial parse report.
- `frontend::parse_sources` parses deterministic source order but stops at the first failing file.
- `CompilerError` carries one stage/message/span, so compiler entrypoints return one frontend
  failure even though the CLI `CliError` container and human/JSON renderers already support a
  vector of diagnostics.
- Semantic, IR, backend, and native stages are not entered after a frontend failure. P162 must
  preserve this boundary.

The token model always appends EOF and parser `peek`/`advance` rely on that sentinel. It does not
preserve newlines. Top-level declaration starters are structurally distinguishable, but block
recovery cannot treat every identifier as a safe new statement without risking a cascade.

## Panic and invariant baseline

Production code has no blanket panic containment. Invariant-dependent `expect` sites remain in the
frontend merge, parser pattern extraction, project graph traversal, package/link scopes, semantic
and IR normalization, diagnostic serialization, and CLI source lookup. Most have a preceding
validation or non-empty construction proof, but current tests do not systematically prove that
malformed user input cannot reach each site.

P163 will classify each site as one of:

1. user-reachable and converted to a stage-owned diagnostic,
2. malformed internal IR guarded by an explicit validator/negative test, or
3. locally proven infallible operation retained with a narrow assertion.

`catch_unwind` is not a substitute for this audit.

## Property and corpus baseline

Existing property-like evidence is narrow and deterministic:

- formatter token/comment parity and all-example idempotence,
- deterministic project/dependency/test discovery,
- repeated release archive byte identity,
- strict generated C and 67-program ASan/UBSan sweep.

There is no arbitrary UTF-8 lexer property, parser token/source mutation property, checked-in
compiler crash corpus, or type/ownership invalid-program generator. P164 will add stable-toolchain
deterministic generators first and preserve every discovered failure as a minimized fixture.

## Performance and reproducibility baseline set

P165 will measure the following repository-owned cases:

| Case | Purpose | Metrics |
| --- | --- | --- |
| `examples/first.mlg` | minimal standalone startup | check/build wall time, generated C and binary bytes |
| `examples/full-expression-cleanup.mlg` | cleanup-heavy language path | check/build/runtime median and output |
| `examples/projects/local-deps/app` | multi-project dependency graph | check/build wall time and generated C bytes |
| `examples/projects/textstats` | standard-library reference CLI | build time, generated C/binary bytes and fixture runtime |

The first machine-readable baseline is observational. A later decision gate sets absolute plus
relative regression thresholds after native CI variance is known.

Current reproducibility evidence covers formatter output, deterministic source/graph order, and
release archives. Generated C appears deterministic but does not yet have a repeated-build byte
gate. Native executable byte identity remains excluded across host/toolchain combinations.

## P162 slice order

### Slice A: top-level recovery and aggregation

- Add an internal parse report that can carry an ordered `Vec<ParseError>` while preserving the
  existing single-error convenience API.
- Recover to the next structurally valid `pub`, `type`, named `func`, contextual `test`, `import`,
  or `package` boundary with mandatory cursor progress.
- Aggregate frontend errors across deterministic project source order, cap each source at 32, and
  emit human/JSON records from the existing shared diagnostic model.
- Do not return a partial program or enter semantic analysis when any parse error exists.

### Slice B: block statement recovery

- Synchronize at `;`, the current block's `}`, or unambiguous statement keywords while tracking
  nested delimiters.
- If an identifier-led statement has no safe boundary, abandon the current block and resume at the
  next top-level declaration instead of guessing.
- Add nested block/function-literal regressions to prove that recovery does not misclassify inner
  tokens as top-level declarations.

### Slice C: cap and compatibility acceptance

- Fix stable source/span order, duplicate suppression, 32-error truncation, human/JSON parity, and
  non-zero exit behavior.
- Keep lexical errors fail-fast until P164 lexer property evidence justifies recoverable lexing.

This ordering implements the approved recovery contract without introducing newline syntax or a
parser-library migration.
