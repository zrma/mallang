# Open Questions: v0.2-project-model

상태: closed

2026-07-14에 추천안을 v0.2 language/project surface로 승인했다.

## 확정된 결정

- Q1: 각 project source는 `package <name>`으로 namespace를 선언한다.
- Q2: `import "project/path"`를 사용하고 마지막 path segment를 기본 qualifier로
  사용한다.
- Q3: declaration은 기본 package-private이며 외부 공개에는 explicit `pub`을
  사용한다.
- Q4: `mallang.toml`이 project root를 정의한다. Directory 입력은 가장 가까운 상위
  manifest를 찾고, direct manifest 입력도 project mode로 처리한다.
- Q5: source는 `src/` 아래에 두고 `src/main.mlg`를 executable entry source로
  사용한다. 같은 directory의 `.mlg` 파일은 하나의 package를 이룬다.

Direct `.mlg` 입력은 project 내부에서도 manifest-free standalone mode를 유지한다.
v0.2에서는 import alias, dot/wildcard import, remote dependency, registry, lockfile,
package initialization hook을 도입하지 않는다.

추가 language surface 결정이 필요해질 때만 새 decision gate를 연다.
