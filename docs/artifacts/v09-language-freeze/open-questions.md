# Open Questions: v0.9 Language Freeze

상태: Q1-Q6 recommendations approved on 2026-07-16

## Q1. Freeze boundary

추천: v0.8에 구현되고 published spec/test로 검증되는 lexical grammar, syntax, type/ownership
rules, standard packages와 CLI behavior를 v1 candidate로 동결한다. 이후 source-visible 변화는
safety/soundness 또는 명백한 spec contradiction을 고치는 경우만 허용한다.

## Q2. Compiler and language version

추천: v0.9까지 compiler package version과 implemented language specification version을 같은
release line으로 유지한다. Source manifest에는 edition/language version을 추가하지 않는다.
`v1.0.0` compiler가 Mallang v1 contract의 최초 stable implementation이다.

## Q3. Compatibility and deprecation

추천: v1 이후 1.x compiler는 valid v1 source를 계속 accept하고 observable semantics를
조용히 바꾸지 않는다. Removal 또는 source-breaking syntax/type change는 다음 major version으로
미루고, 가능한 경우 한 minor 이상 diagnostic deprecation을 먼저 제공한다.

## Q4. Normative conformance

추천: prose 존재만으로 완료 처리하지 않는다. 각 normative rule에 stable ID를 부여하고 Rust
test, `.mlg` fixture, script 또는 명시적인 manual verification evidence 중 하나를 연결한다.
Evidence 없는 rule은 v1 blocker다.

## Q5. Migration and dogfood

추천: migration guide는 실제로 제거된 borrow aliases, 확정된 `con`/`mut`, project/test/standard
library progression을 다룬다. Existing multi-module `textstats`를 representative dogfood로
사용하며 새 showcase feature를 만들지 않는다.

## Q6. RC, upgrade and rollback

추천: `v0.9.0` freeze release 뒤 `v1.0.0-rc.1`을 별도 signed prerelease로 게시한다. Clean
install, explicit upgrade, pinned-version rollback과 textstats rebuild를 supported platforms에서
검증한 뒤 stable tag를 허용한다.

## Approval Decision

Q1-Q6 추천안은 2026-07-16 승인됐다. v0.9에서는 feature-freeze를 깨는 새 surface를 추가하지
않고 `spec.md`의 P167-P172를 순서대로 진행한다.
