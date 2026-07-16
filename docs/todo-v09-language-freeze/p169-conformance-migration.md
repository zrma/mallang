# P169: Conformance and Migration Map

상태: complete (2026-07-17)

## Conformance result

`docs/conformance/v1-rules.json`은 candidate contract의 98개 rule을 23개 evidence
profile에 정확히 한 번씩 배치한다. 63개 evidence item은 다음 네 종류만 허용한다.

- executable repository script
- checked-in `.mlg` fixture
- source tree에 실제 존재하는 Rust test symbol
- repository-relative explicit verification command

`scripts/check-v1-conformance.py`는 contract와 manifest rule exact-set, duplicate/unmapped/
unknown rule, profile/evidence schema, repository-relative path, executable bit, fixture suffix,
Rust test symbol, compatibility policy heading과 migration guide 존재를 검사한다. Canonical
`scripts/check.sh`가 이 checker를 가장 먼저 실행하므로 rule 추가나 삭제는 evidence map을
같이 갱신하지 않으면 실패한다.

## Migration result

`docs/MIGRATION_V1.md`는 다음 migration을 하나의 공개 문서로 통합한다.

- pre-v0.1 bootstrap의 suffix `in`/`mut`와 call-site `in`을 prefix `con`/`mut`로 변환
- non-Copy range를 index-only range와 indexed `con`/`mut` call로 변환
- standalone source를 `mallang.toml`/package/import/pub project로 전환
- local dependency exact-name path와 transitive import boundary
- built-in ADT spelling 유지와 user enum qualified constructor/pattern
- v0.6 `std` project name과 `__mlg_` identifier reservation
- project tests, JSON diagnostics, formatter와 explicit-version install workflow

Published v0.1.0 language form은 v0.8.0까지 의도적으로 제거되지 않았음을 release/spec
history와 대조했다. Borrow alias는 published compatibility surface가 아니라 bootstrap
experiment였으므로 legacy alias를 추가하지 않는다.

## Executable acceptance

`scripts/check-v1-migration.sh`는 canonical fixture를 check/build/run해
`kim`, `lee`, `a`, `b` output을 검증하고 다음 source를 stable diagnostic으로 거부한다.

- `name in T`
- `name mut T`
- `in expr`
- `for i, con value := range values`

이 acceptance와 conformance checker는 debug canonical gate에 포함된다. P170은 이 rule
map을 representative `textstats` clean dogfood workflow로 소비한다.
