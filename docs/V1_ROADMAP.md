# Mallang v1 Roadmap

상태: planned

기준 릴리스: `v0.1.0`

목표 릴리스: `v1.0.0`

이 문서는 Mallang `v0.2.0`부터 `v1.0.0`까지의 장기 마일스톤 경계를
정의한다. 현재 구현된 언어 동작은 `SPEC.md`, compiler source, tests, examples가
소유하며, 이 문서의 미완료 항목을 현재 지원 기능으로 해석하지 않는다.

## v1의 의미

Mallang `v1.0.0`은 기능의 총량보다 안정성 약속을 뜻한다. 다음 조건을 만족하면
v1로 간주한다.

- 여러 source file과 module로 구성된 중간 규모의 native CLI 프로그램을 작성할
  수 있다.
- ownership, borrow, cleanup 규칙이 v1 surface 전체에 적용되며 알려진 memory-safety
  위반 경로가 없다.
- 함수형 value style을 실제 library code에 사용할 수 있도록 closure, generic data
  type, pattern matching을 제공한다.
- arguments, file I/O, strings, collections, error propagation을 포함한 최소 standard
  library가 있다.
- `mlg check`, `mlg build`, `mlg run`, `mlg test`, formatter가 project 단위로
  동작한다.
- 지원 platform, language specification, compatibility policy와 release gate가
  문서화되어 있다.
- C backend가 v1 language surface 전체를 지원한다. LLVM, Cranelift 또는
  self-hosting compiler는 v1 조건이 아니다.

## 마일스톤 요약

| 버전 | 주제 | 핵심 결과 |
| --- | --- | --- |
| `v0.2.0` | Projects and Modules | multi-file project를 check/build/run한다. |
| `v0.3.0` | Functions and Closures | first-class function과 safe closure를 지원한다. |
| `v0.4.0` | Generic Data Model | user-defined ADT와 generic type/function을 지원한다. |
| `v0.5.0` | Ownership and Runtime | v1 memory model과 owned heap value 경계를 닫는다. |
| `v0.6.0` | Standard Library | 실제 CLI 프로그램에 필요한 core library를 제공한다. |
| `v0.7.0` | Tooling and Platforms | test, format, install, multi-platform workflow를 제공한다. |
| `v0.8.0` | Compiler Hardening | diagnostics, robustness, performance를 release 수준으로 높인다. |
| `v0.9.0` | Language Freeze | v1 문법, semantics, compatibility contract를 동결한다. |
| `v1.0.0` | Stable Release | 검증된 v1 contract와 배포물을 공개한다. |

버전 번호는 목표 날짜가 아니라 완료된 capability를 나타낸다. 각 마일스톤은
필요하면 여러 patch release로 나눌 수 있지만, 다음 minor version은 현재
마일스톤의 완료 조건을 통과한 뒤에만 시작한다.

## 공통 완료 규칙

모든 마일스톤은 다음 규칙을 공유한다.

- syntax만 추가하지 않고 lexer/parser, semantic, ownership, IR/backend, native
  execution을 필요한 범위까지 end-to-end로 연결한다.
- safety 기능은 성공 예제와 rejection regression을 함께 추가한다.
- generated C는 strict warning gate와 관련 sanitizer gate를 통과한다.
- public CLI 동작은 release binary smoke로 검증한다.
- `SPEC.md`, roadmap, handoff에서 planned와 implemented 상태를 같은 change에서
  갱신한다.
- 서로 호환되지 않는 language surface 후보가 있으면 구현 전에
  `docs/ESCALATION_POLICY.md`에 따라 decision gate를 연다.
- 새로운 backend 도입은 현재 C backend가 해당 마일스톤을 막는다는 증거가 있을
  때만 검토한다.

## v0.2.0: Projects and Modules

상태: implementation complete, release pending

목표: 단일 source file 언어에서 multi-file project 언어로 전환한다.

범위:

- `package <name>`, `import "project/path"`, explicit `pub` surface를 구현한다.
- `mallang.toml`, `src/main.mlg`, directory package layout을 구현한다.
- 여러 source file의 declaration graph와 deterministic build order를 만든다.
- file-aware source span과 cross-file diagnostics를 추가한다.
- duplicate declaration, unresolved import, visibility violation, module cycle의 동작을
  명시하고 검증한다.
- lexer/parser 장기 구조를 평가하고 hand-written parser 유지, module split, lexer
  library 도입 여부를 기록한다.

완료 조건:

- 최소 두 module로 구성된 project를 `mlg check`, `mlg build`, `mlg run`으로
  처리한다.
- cross-file function, struct, method 호출이 native binary에서 실행된다.
- invalid module graph가 file과 span을 포함한 안정적인 diagnostic으로 거부된다.
- 기존 single-file source는 명시된 compatibility rule에 따라 계속 동작한다.

제외:

- remote package registry
- third-party dependency resolution
- interfaces와 dynamic dispatch
- first-class references

## v0.3.0: Functions and Closures

상태: implementation complete, release pending

목표: Mallang의 함수형 value style을 first-class function까지 확장한다.

범위:

- function type, function value, higher-order call을 정의한다.
- closure literal과 capture 규칙을 정의한다.
- owned capture, `con`/`mut` call access와 escaping closure의 허용 범위를 결정한다.
- closure environment의 typed IR, C layout, cleanup을 구현한다.
- pipeline과 collection operation에서 function value를 사용할 수 있게 한다.

완료 조건:

- function을 parameter와 return value로 전달할 수 있다.
- captured value의 move, borrow, mutation이 일반 ownership 규칙과 일치한다.
- escaping closure에서 dangling reference가 생성되지 않는다.
- closure를 사용하는 native positive smoke와 invalid capture rejection이 있다.

제외:

- async function과 coroutine
- implicit shared mutable capture
- runtime reflection

## v0.4.0: Generic Data Model

상태: implementation complete, release pending

목표: built-in `Option`/`Result` 전용 모델을 일반적인 user-defined data model로
확장한다.

범위:

- user-defined enum 또는 sum type surface를 정의한다.
- generic function과 generic type을 지원한다.
- monomorphization 또는 동등한 static specialization 전략을 고정한다.
- nested pattern, payload destructuring과 필요한 pattern diagnostics를 추가한다.
- built-in `Option[T]`와 `Result[T, E]`가 일반 generic ADT 규칙과 일관되게
  동작하도록 정리한다.
- interface/trait가 실제 generic use case에 필요한지 별도 decision gate에서
  평가한다.

완료 조건:

- user-defined generic collection 또는 ADT가 두 개 이상의 concrete type으로
  native compile된다.
- exhaustive `match`가 user-defined ADT에서도 동작한다.
- invalid type argument, constructor, pattern을 source diagnostic으로 거부한다.
- generic ownership와 cleanup이 concrete non-copy payload에서 검증된다.

제외:

- specialization syntax
- higher-kinded types
- dynamic dispatch는 별도 승인 없이는 포함하지 않는다.

## v0.5.0: Ownership and Runtime

목표: v1 language surface 전체에 적용할 memory model과 runtime ownership 경계를
닫는다.

범위:

- generic value, closure environment, ADT에 move/borrow/drop 규칙을 확장한다.
- owned recursive data를 위한 safe heap value abstraction을 제공한다.
- struct/ADT field take와 partial move 규칙을 일반화하거나 명시적으로 제한한다.
- return, branch, loop, early exit, overwrite의 cleanup 정확성을 완성한다.
- mutable range value와 by-reference iteration의 최종 v1 경계를 결정한다.
- first-class reference와 statement-spanning borrow가 v1에 필요한지 use case로
  재평가한다.

기본 방향:

- first-class reference를 요구하는 구체적인 v1 use case가 없다면 `con`/`mut`
  borrow는 call-scoped로 유지한다.
- user-visible lifetime syntax와 raw pointer syntax는 도입하지 않는다.
- recursive ownership은 pointer syntax 대신 compiler 또는 standard-library가
  소유하는 safe abstraction으로 제공한다.

완료 조건:

- v1 후보 type 전체에 drop classification과 cleanup path가 정의되어 있다.
- known use-after-move, double-drop, leak, dangling borrow 경로가 regression으로
  고정되어 있다.
- cleanup-heavy generated C가 sanitizer sweep을 통과한다.
- memory model이 implementation과 language specification에서 동일하게 설명된다.

## v0.6.0: Standard Library

목표: compiler demo를 넘어 실제 native CLI 프로그램을 작성할 수 있는 최소
library를 제공한다.

범위:

- program arguments와 environment access
- file and stream I/O
- string conversion, search, split, join 등 기본 text operation
- owned slice 보강과 key-value collection
- `Option`/`Result` 기반 error propagation과 최소 error type convention
- core, standard library, runtime implementation의 module 경계
- platform-specific runtime code를 안전한 Mallang API 뒤에 격리하는 규칙

완료 조건:

- argument로 입력 파일을 받아 읽고, 변환하고, 결과 파일 또는 stdout에 쓰는
  multi-module CLI example이 있다.
- expected runtime failure가 `Result`로 전달되고 process exit behavior가
  검증된다.
- standard library API가 ownership mode와 failure behavior를 문서화한다.
- library example 전체가 supported platform의 native smoke에 포함된다.

제외:

- public package registry
- full networking stack
- user-visible unsafe FFI

## v0.7.0: Tooling and Platforms

목표: 반복 개발과 배포에 필요한 기본 developer workflow를 제공한다.

범위:

- project-aware `mlg test`
- canonical formatter와 `mlg fmt --check`
- project check/build/run의 안정적인 CLI contract
- local 또는 path dependency의 최소 workflow
- release binary 설치와 update 경로
- 최소 macOS와 Linux target matrix
- editor integration을 위한 machine-readable diagnostics
- basic LSP는 release blocker가 아닌 권장 목표로 평가한다.

완료 조건:

- 새 project 생성부터 format, test, release build까지 문서화된 한 경로가 있다.
- formatter output이 deterministic하고 idempotent하다.
- 최소 macOS arm64와 Linux x86_64에서 representative project가 build/run된다.
- release artifact가 clean environment에서 설치되고 smoke test를 통과한다.

제외:

- centralized package registry
- 모든 editor를 위한 plugin
- 모든 OS와 architecture 지원

## v0.8.0: Compiler Hardening

목표: 언어 기능 추가보다 compiler 품질, 진단, 성능을 우선하는 안정화 구간을
완료한다.

범위:

- parser error recovery와 한 번의 check에서 유용한 복수 diagnostic 제공
- compiler panic과 malformed IR 방어
- lexer/parser, type checker, ownership checker의 fuzz 또는 property test
- compile time, generated C size, runtime overhead baseline
- deterministic output과 reproducible-build 범위
- parser/lexer library migration은 측정된 유지보수 또는 diagnostic 이점이 있을 때만
  수행한다.

완료 조건:

- known compiler crash corpus가 모두 non-zero diagnostic으로 처리된다.
- representative project의 compile/runtime baseline과 regression threshold가 있다.
- v1 candidate example 전체가 warning, sanitizer, release smoke gate를 통과한다.
- unsupported feature가 compiler panic 대신 명확한 diagnostic을 낸다.

## v0.9.0: Language Freeze

목표: 새로운 핵심 기능을 멈추고 v1 compatibility contract를 검증한다.

범위:

- lexical grammar, syntax, type system, ownership, runtime behavior를 v1 spec으로
  통합한다.
- compiler version과 language version의 관계를 정의한다.
- breaking change, deprecation, edition 또는 major-version 정책을 확정한다.
- conformance test suite와 0.x migration guide를 작성한다.
- 실제 multi-module CLI project로 dogfooding한다.
- `v1.0.0-rc` release와 install/upgrade rollback rehearsal를 수행한다.

완료 조건:

- unresolved v1 language decision gate가 없다.
- feature freeze 이후에는 bug, diagnostics, documentation, compatibility 수정만
  허용한다.
- v1 spec의 모든 normative rule이 test 또는 명시적인 verification evidence와
  연결된다.
- 지원 platform release artifact와 representative project가 RC gate를 통과한다.

## v1.0.0: Stable Release

목표: 검증된 v1 language contract와 toolchain을 stable로 공개한다.

범위:

- v1 specification, standard library reference, CLI guide, migration guide 공개
- supported platform release artifact와 checksum 제공
- compatibility policy와 security/reporting boundary 공개
- clean-install, build, run, test, format, upgrade smoke 수행
- release provenance와 verification evidence 보존

완료 조건:

- v0.9 feature freeze 이후 해결되지 않은 release blocker가 없다.
- v1 representative CLI project가 clean environment에서 수정 없이 동작한다.
- v1 safety and compiler-hardening gate가 release binary로 통과한다.
- `v1.0.0` 이후 1.x에서 지킬 source compatibility 범위가 명확하다.

`v1.0.0`은 새로운 언어 기능을 추가하는 마일스톤이 아니다. v0.9에서 동결한
contract를 검증하고 배포하는 마일스톤이다.

## 기본적으로 v1 이후로 미루는 항목

다음 항목은 앞선 마일스톤의 구체적인 blocker가 되지 않는 한 v1 범위에 넣지
않는다.

- goroutine, async runtime, coroutine
- raw pointer, address-of, user-visible unsafe block
- first-class reference와 user-visible lifetime
- interface/dynamic dispatch 전체 체계
- public C interop 또는 user-visible FFI
- LLVM 또는 Cranelift backend 전환
- self-hosting compiler
- centralized package registry
- JIT, garbage collector, runtime reflection

## 주요 결정 게이트

| 시점 | 결정 |
| --- | --- |
| `v0.2` 시작 전 | module/import/visibility syntax와 project manifest 경계 |
| `v0.3` 시작 전 | closure literal, capture mode, escaping closure 규칙 |
| `v0.4` 시작 전 | enum/generic syntax와 specialization strategy |
| `v0.5` 시작 전 | owned heap abstraction, partial move, first-class reference 필요성 |
| `v0.7` 시작 전 | supported target matrix와 installation contract |
| `v0.9` 시작 전 | v1 compatibility, deprecation, edition policy |

각 decision은 독립된 `docs/todo-*/spec.md`와 `open-questions.md`에서 닫은 뒤
implementation milestone로 전환한다.
