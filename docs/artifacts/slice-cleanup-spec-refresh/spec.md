# Spec: slice-cleanup-spec-refresh

Status: complete; historical milestone record

## 목표

- `SPEC.md`의 slice cleanup 설명을 현재 구현 상태에 맞춘다.
- 아직 미룬 언어 기능과 이미 구현된 cleanup lowering을 같은 future bucket에
  두지 않는다.

## 결정

- Drop helper emission, explicit internal drop IR, automatic cleanup insertion은
  current implemented model로 설명한다.
- Mutable range values, by-reference iteration, borrowed slice views,
  first-class references, shared backing buffers는 deferred rule로 유지한다.

## 범위

- `SPEC.md`의 slice cleanup/future rule wording 정리.
- `ROADMAP.md`와 `docs/ROADMAP.md`에 P78 기록 추가.

## 제외

- 새 cleanup behavior 구현.
- Runtime state tracking 완화.
- Borrowed temporary cleanup 또는 first-class reference 설계.

## C-체크리스트

| ID | 상태 | Verify command | 작업 항목 |
| --- | --- | --- | --- |
| C1 | done | `! rg -n "before automatic|Initial automatic|may also emit" SPEC.md` | stale staging wording removed |
| C2 | done | `scripts/check.sh` | full repo smoke |

## 완료 기준

- C-체크리스트가 완료되고 검증 명령이 통과한다.
