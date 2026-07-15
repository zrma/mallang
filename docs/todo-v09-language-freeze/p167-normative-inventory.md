# P167: Normative Contract Inventory

상태: complete (2026-07-16)

## 결과

`docs/V1_LANGUAGE_CONTRACT.md`에 v1 candidate surface를 stable rule ID로 통합했다.
범위는 source/lexical, project/package, declaration/type, function/closure,
expression/control flow, ownership, standard library, CLI/diagnostic, native runtime과
distribution이다.

Rule ID는 `<version>-<area>-<number>` 형식이며 삭제하거나 다른 의미로 재사용하지 않는다.
P168 compatibility policy와 P169 conformance map은 이 ID를 기준으로 작성한다.

## Current-source inventory

| Source | 확인 결과 | P167 조치 |
| --- | --- | --- |
| `SPEC.md` | v0.2-v0.8 project, tooling, hardening과 core language prose가 섞여 있고 rule ID가 없었다. | candidate rule table로 통합하고 현재 구현과 충돌한 type/ADT/match 문구를 교정했다. |
| `docs/STANDARD_LIBRARY.md` | 여섯 package의 exact signature와 ownership/failure semantics가 별도 reference에 있었다. | `V1-STD-*` rule에서 reference 전체를 normative detail owner로 고정했다. |
| `src/main.rs`, README, release checks | stable user command와 inspection command, exit behavior가 여러 위치에 분산돼 있었다. | `V1-CLI-*`, `V1-DIAG-*`로 stable surface와 non-serialized inspection stdout 경계를 분리했다. |
| release workflow and artifact checks | supported target, archive, installer와 reproducibility contract가 release section/script에 분산돼 있었다. | `V1-RUN-*`으로 source semantics와 implementation/distribution contract를 분리했다. |

## Drift found and resolved

| Drift | Current implementation evidence | Resolution |
| --- | --- | --- |
| Copy/move 목록이 `int bool`과 `string struct`까지만 열거됐다. | semantic type classification은 `unit`, conditional `Option`/`Result`, move-only array/slice/struct/enum/function을 포함한다. | `SPEC.md` type classification과 `V1-TYP-*`를 current model로 갱신했다. |
| `Option and Result`가 user enum과 nested pattern을 미지원이라고 적었다. | generic user enum, qualified constructor, nested pattern과 exhaustive checker가 v0.4부터 release gate에 있다. | stale exclusion을 제거하고 user ADT 및 recursive/productive boundary를 명시했다. |
| `match` 규칙이 built-in `Option`/`Result` exact-arm model만 설명했다. | user enum, wildcard, nested pattern, duplicate/unreachable/non-exhaustive diagnostics가 구현돼 있다. | `SPEC.md` match rule을 unified ADT model로 교정했다. |
| `lex`/`parse`/`ir`의 공개 여부와 stdout 안정성 경계가 명시되지 않았다. | 세 command는 help와 release smoke에 노출되지만 출력 parser API는 없다. | command는 유지하되 successful inspection stdout은 stable serialization이 아니라고 고정했다. |

## Remaining freeze blockers

| Blocker | Owner milestone | Completion condition |
| --- | --- | --- |
| Compiler/language version 관계와 1.x compatibility, deprecation, breaking-change 규칙이 아직 candidate rule에 연결되지 않았다. | P168 | 공개 compatibility policy가 rule ID와 release line을 명시한다. |
| Rule ID별 executable/manual evidence 연결이 없다. | P169 | 모든 `V1-*` rule이 test, fixture, script 또는 explicit command에 연결되고 unmapped count가 0이다. |
| v0.1-v0.8에서 제거된 syntax/API migration이 여러 milestone 문서에 흩어져 있다. | P169 | 하나의 migration guide와 valid/invalid migration fixtures로 통합한다. |
| Representative project가 frozen contract와 clean installation을 함께 소비한다는 반복 evidence가 필요하다. | P170 | `textstats` clean format/check/test/build/run workflow가 통과한다. |

별도 EBNF, stable generated C ABI, exact human diagnostic message catalog와 numerical performance
threshold는 v1 blocker로 추가하지 않는다. Parser fixtures와 conformance map이 accepted grammar를
고정하고, diagnostic schema/semantic fields만 compatibility surface이며, C/native layout과
performance observation은 명시적으로 implementation evidence다.

## Verification

- candidate document의 rule ID는 area별로 단조 증가하고 중복이 없다.
- candidate가 참조하는 detail owner는 모두 repository tracked file이다.
- published v0.8 source examples와 현재 semantic type classification을 대조했다.
- P168/P169/P170에 남긴 blocker 외에는 unresolved source-visible decision이 없다.
