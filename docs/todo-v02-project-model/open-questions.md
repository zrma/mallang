# Open Questions: v0.2-project-model

아래 질문은 서로 호환되지 않는 language/project surface를 정하므로 사용자 결정이
필요하다. 한꺼번에 **추천안 승인**으로 닫을 수 있다.

## Q1. Source file의 namespace 선언

- **A. `package <name>` (추천)**: Go와 익숙한 형태이며 같은 directory의 여러 파일을
  자연스럽게 하나의 compilation unit으로 묶는다.
- B. path-derived module: 선언은 줄지만 파일 이동이 namespace 변경을 암묵적으로
  일으킨다.
- C. `module <path>`: namespace가 명확하지만 Go-like surface에서 더 멀어진다.

## Q2. Import 문법

- **A. `import "project/path"` (추천)**: Go와 유사하고 path 경계가 문자열로
  명확하다. 기본 qualifier는 마지막 path segment다.
- B. `import project.path`: 간결하지만 identifier와 path grammar가 결합된다.
- C. selective import: 초기 문법과 name resolution 복잡도가 커진다.

v0.2에서는 alias, dot import, wildcard import를 제외하는 것을 추천한다.

## Q3. Visibility 문법

- **A. explicit `pub` (추천)**: declaration 이름과 visibility를 분리하고 검색하기
  쉽다. 표시가 없으면 package-private다.
- B. Go식 대문자 export: 키워드는 줄지만 이름 변경이 API 변경을 일으킨다.
- C. package export list: 선언부는 단순하지만 API 정의가 declaration에서 멀어진다.

## Q4. Project root와 manifest

- **A. `mallang.toml` (추천)**: root, project name, 이후 local dependency와 tool
  option을 확장할 자리가 생긴다.
- B. manifest 없이 `src/main.mlg` 탐색: 초기 파일은 줄지만 root와 project identity가
  불명확해진다.

manifest는 project mode에서만 요구하고 standalone `.mlg` 명령은 계속 허용하는 것을
추천한다.

## Q5. 기본 source layout

- **A. `src/main.mlg`와 하위 package directory (추천)**: executable entrypoint가
  안정적이고 source와 project metadata가 분리된다.
- B. project root의 `.mlg` 파일: 작은 project는 단순하지만 metadata와 source가
  섞이고 규모가 커질수록 discovery 규칙이 불명확해진다.

