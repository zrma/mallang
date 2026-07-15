# Spec: v0.7 Tooling and Platforms

상태: P154-P158 complete; P159 next

## Goal

Mallang의 project model을 반복 개발과 clean installation에 사용할 수 있는 하나의
developer workflow로 연결한다. Language feature 확장보다 deterministic tooling,
machine-readable integration과 supported platform delivery를 우선한다.

## Candidate scope

- Project-aware `mlg test` discovery, execution, failure reporting
- Comment-preserving canonical formatter와 `mlg fmt --check`
- Local path dependency manifest와 deterministic package graph
- Stable human diagnostic와 versioned machine-readable diagnostic output
- macOS arm64/Linux x86_64 release artifact, install, update와 clean-host smoke
- Basic LSP feasibility; release blocker 여부는 decision gate에서 확정

## Proposed implementation order

### P154: Tooling Decision Gate

- Formatter trivia/comment preservation, test model, path dependency graph,
  diagnostic schema와 release distribution feasibility를 inventory한다.
- `open-questions.md`의 compatibility choices를 승인받는다.

진행:

- [x] current compiler/tooling feasibility inventory
- [x] Q1-Q6 compatibility and implementation impact 기록
- [x] Q1-Q6 사용자 승인

### P155: Canonical Formatter

- Deterministic and idempotent source formatting과 `--check`를 구현한다.
- Comments와 source meaning preservation을 golden/property test로 고정한다.

진행:

- [x] raw source span 기반 lossless token/trivia formatter
- [x] 4-space/LF/final-newline/max-one-blank-line canonical style
- [x] direct file/project write mode와 no-write `--check`
- [x] project parse failure 전 파일 no-write와 deterministic relative path output
- [x] token/comment parity, checked-in examples idempotence와 multiline comment regressions
- [x] debug canonical gate와 optimized release binary formatter smoke

### P156: Project Test Workflow

- Test discovery, assertion/failure contract, filtering과 exit status를 구현한다.
- Multi-package test fixtures와 deterministic output을 연결한다.

진행:

- [x] current parser/project/compiler/native runner feasibility inventory
- [x] optional `tests/` recursive deterministic discovery API
- [x] `p156-test-workflow.md` Q1-Q6 recommendation and acceptance matrix
- [x] P156 Q1-Q6 사용자 승인
- [x] parser부터 native runner까지 end-to-end implementation

### P157: Local Path Dependencies

- Manifest path dependency, cross-project package identity와 cycle diagnostics를 구현한다.
- Central registry, network resolution과 lockfile은 범위 밖으로 유지한다.

진행:

- [x] P154 Q3 local dependency recommendation approval
- [x] exact manifest, graph, import와 library command contract
- [x] recursive project graph부터 native CLI까지 end-to-end implementation

### P158: Machine-readable Diagnostics

- Versioned JSON diagnostic schema와 human output parity를 구현한다.
- Editor consumer fixture를 추가하고 basic LSP 진행 여부를 재평가한다.

진행:

- [x] `mallang.diagnostic.v1` JSON Lines schema와 stable stage vocabulary
- [x] shared human/JSON renderer와 global `--diagnostic-format`
- [x] project/dependency source path 및 UTF-8 span normalization
- [x] JSONL consumer fixture와 debug/release smoke
- [x] full LSP를 v0.7 blocker에서 제외하고 P160 decision gate로 보류

### P159: Release Artifacts and Installation

- macOS arm64/Linux x86_64 artifacts, checksums, install/update contract와 clean-host
  smoke를 구현한다.

### P160: v0.7 Acceptance

- New project에서 format, test, release build, install까지 canonical workflow를 닫는다.
- Documentation, CI matrix와 v0.8 hardening decision gate를 동기화한다.

## Excluded

- Central package registry and remote dependency solver
- Windows support declaration
- Editor-specific plugins
- Language-server protocol as a release blocker before feasibility evidence
