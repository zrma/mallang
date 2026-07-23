# Spec: borrow-mode-alias-rejection-regression

Status: complete; historical milestone record

## 목표

- v0 borrow surface를 `con`/`mut` prefix grammar로만 유지한다.
- 하위 호환 대상이 없는 초기 언어이므로 legacy alias나 실험 후보 syntax를 열지
  않는다.

## 결정

- 허용 문법은 `con name T`, `mut name T`, `con expr`, `mut expr`뿐이다.
- `in`은 keyword가 아니라 identifier로 남긴다.
- `name in T`, `name mut T`, `in expr`는 v0 문법이 아니다.

## 범위

- Parser regression: suffix read-borrow parameter candidate reject.
- Parser regression: suffix mutable-borrow parameter candidate reject.
- Parser regression: call-site `in expr` borrow alias candidate reject.

## 제외

- 새 diagnostic 문구 설계.
- Migration warning 또는 compatibility alias.
- `const`/`ref`/`borrow` 같은 다른 naming 후보 재검토.

## C-체크리스트

| ID | 상태 | Verify command | 작업 항목 |
| --- | --- | --- | --- |
| C1 | done | `cargo test --all-targets parser::tests::rejects_legacy` | parser legacy alias rejection regression |
| C2 | done | `scripts/check.sh` | full repo smoke |

## 완료 기준

- C-체크리스트가 완료되고 검증 명령이 통과한다.
