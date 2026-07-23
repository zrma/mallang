# Open Questions

구현 마일스톤을 열 때 아래 범위를 확정한다.

- 첫 배포를 어느 compatible 1.x minor에 포함할지.
- `match` arm block만 먼저 열지, 같은 내부 divergence 모델을 `if`
  expression에도 동시에 적용할지. 권장안은 `match`만 먼저 여는 것이다.
- Compiler orchestration에서 먼저 분리할 helper와 native acceptance case.

`?`, exception, public `Never`, implicit error conversion은 선택지에 포함하지
않는다.
