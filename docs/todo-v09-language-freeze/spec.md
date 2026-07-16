# Spec: v0.9 Language Freeze

상태: complete; v0.9.0 freeze, v1.0.0-rc.1 rehearsal와 v1.0.0 stable release 완료

## Goal

v0.8의 lexical grammar, syntax, types, ownership, standard packages와 observable runtime
behavior를 v1 candidate로 동결한다. 새 핵심 기능 대신 normative specification,
compatibility, conformance, migration, dogfood와 release rehearsal의 빈틈을 닫는다.

## Implementation order

### P167: Normative Contract Inventory

- [x] 현재 `SPEC.md`, standard library와 CLI contract를 rule ID가 있는 v1 candidate 문서로 통합한다.
- [x] 구현됐지만 normative wording 또는 evidence가 없는 항목을 inventory한다.

### P168: Version and Compatibility Policy

- [x] compiler/language version 관계, v1 source compatibility, deprecation과 breaking-change 규칙을
  공개 문서로 고정한다.
- [x] v1에는 edition field나 per-project language-version switch를 추가하지 않는다.

### P169: Conformance and Migration Map

- [x] 각 normative rule을 test, fixture 또는 explicit verification command에 연결한다.
- [x] v0.1-v0.8에서 제거되거나 정규화된 syntax/API migration을 한 문서로 통합한다.

### P170: Representative Dogfood

- [x] `examples/projects/textstats`를 clean install부터 format/check/test/build/run까지 반복 실행한다.
- [x] 발견된 문제를 frozen surface 변경 없이 bug, diagnostic, documentation과 test gap으로
  분류한다.

### P171: v0.9 Acceptance and Release

- [x] freeze 이후 change audit, conformance completeness와 supported-platform artifact를 검증한다.
- [x] `v0.9.0`을 language-freeze release로 게시한다.

### P172: v1 RC and Rollback Rehearsal

- [x] `v1.0.0-rc.1` clean install, v0.9 upgrade, explicit rollback과 representative project를
  supported platforms에서 검증한다.
- [x] unresolved blocker가 없을 때 stable v1 release milestone으로 인계한다.

## Excluded

- new language feature or standard package surface
- first-class references, user-visible lifetime, interface or dynamic dispatch
- edition syntax or manifest language-version switch before concrete compatibility need
- parser/backend replacement, self-hosting or new platform declaration
- numerical performance threshold without repeated supported-platform samples
