# P168: Version and Compatibility Policy

상태: complete (2026-07-17)

## 결정

- Compiler package, implemented language contract, compiler-owned standard packages,
  installer와 archive는 하나의 semantic version을 사용한다.
- `v0.9.0`은 v1 candidate freeze release이고 `v1.0.0`이 Mallang v1의 최초 stable
  implementation이다.
- 모든 v1.x compiler는 하나의 Mallang v1 language contract를 구현한다.
- v1 source와 `mallang.toml`에는 edition, language-version field, pragma 또는 per-project
  compatibility switch를 추가하지 않는다.
- valid v1 source acceptance와 observable semantics는 1.x 전체에서 유지한다.
- Patch/minor는 backward-compatible change만 허용하고, source/type/ownership/API/CLI/target
  break는 다음 major로 미룬다.
- Deprecation은 v1.x에서 source를 계속 accept하고 replacement와 major removal을 문서화한다.
- Memory soundness 또는 security defect는 narrow rejection, rule ID, regression, migration과
  release note가 있을 때만 compatibility exception을 허용한다.

공개 정책은 `docs/COMPATIBILITY.md`, normative index는 `V1-COMP-001`부터
`V1-COMP-013`까지가 소유한다.

## Compatibility boundary

1.x에서 보장하는 범위는 source acceptance, evaluation/ownership/cleanup semantics,
standard API signature와 failure behavior, stable CLI/diagnostic schema, supported target,
artifact와 installer contract다.

Exact human diagnostic wording, successful inspection command stdout, generated C spelling,
native ABI/layout, compiler performance, native binary bytes와 서로 다른 compiler version의
archive bytes는 보장하지 않는다. 같은 compiler/input/options/host scope의 reproducibility는
별도 `V1-RUN-004` rule을 계속 따른다.

## Verification

- Q2 compiler/language relation을 `V1-COMP-001`/`002`로 고정했다.
- Q3 source compatibility/deprecation을 `V1-COMP-003`-`011`로 고정했다.
- Edition 제외 결정을 `V1-COMP-012`로 고정했다.
- Implementation detail과 stable contract 경계를 `V1-COMP-013`으로 고정했다.
- P169 conformance map이 mapping해야 할 contract rule은 총 98개다.
