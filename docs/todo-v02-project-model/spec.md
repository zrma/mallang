# Spec: v0.2-project-model

상태: decision gate open

## 목표

- Mallang의 단일 source file 실행 모델을 multi-file project 모델로 확장하기 전에
  package, import, visibility, project root 계약을 고정한다.
- 기존 `.mlg` 단일 파일 명령을 깨지 않으면서 project 단위 `check`, `build`,
  `run`으로 확장할 수 있는 최소 surface를 정한다.

## Decision 시작 시 제약

- 현재 parser는 top-level `type`과 `func` 선언만 받는다.
- 현재 CLI는 모든 명령에서 정확히 하나의 `.mlg` source file을 받는다.
- source span은 file identity 없이 byte range만 표현했다.
- v0.2에는 remote package registry와 third-party dependency resolution을 넣지 않는다.

## 현재 구현 기반

- `SourceId`가 모든 token과 AST/IR span에 전파된다.
- `SourceMap`이 여러 source file의 path, text, line start를 소유한다.
- `parse_sources`가 여러 파일의 declaration을 입력 순서대로 하나의 compilation
  unit으로 합치고 원본 파일별 span을 보존한다.
- `check_sources`, `lower_sources`, `generate_c_sources`가 같은 source 집합을
  semantic, IR, C backend까지 전달하고 stage별 error를 보존한다.
- CLI frontend diagnostic은 source path와 1-based line/column을 출력한다.
- 기존 `lex`와 `parse` API는 anonymous source를 사용하는 compatibility wrapper로
  유지된다.

## 추천안

아래 조합을 v0.2 기본안으로 추천하되, language surface이므로 사용자 승인 전에는
확정하거나 parser에 구현하지 않는다.

- package 단위: 같은 directory의 `.mlg` 파일을 하나의 package로 묶고 각 파일에
  `package <name>`을 선언한다.
- import: Go와 유사한 `import "<project>/<path>"`를 사용하고 마지막 path segment를
  기본 qualifier로 사용한다.
- visibility: declaration은 기본 private이며 외부 package에 공개할 declaration만
  `pub`으로 표시한다. 대문자 이름을 visibility 신호로 사용하지 않는다.
- project root: `mallang.toml`이 있는 가장 가까운 상위 directory로 정한다.
- source layout: project source는 `src/` 아래에 두고 executable entry package는
  `src/main.mlg`의 `package main`과 `func main()`으로 찾는다.
- single-file compatibility: 명령 인자가 `.mlg` 파일이면 manifest 없이 기존
  standalone 동작을 유지하고, directory 또는 manifest이면 project mode로 처리한다.
- module graph: import path를 정규화해 deterministic order로 처리하고 모든 cycle을
  v0.2에서는 diagnostic으로 거부한다.

예상 surface:

```mlg
package main

import "hello/greet"

func main() {
    greet.Print()
}
```

```mlg
package greet

pub func Print() {
    print("hello")
}
```

## Parser 방향

- v0.2에서는 현재 hand-written lexer와 Pratt parser를 유지한다.
- package/import grammar를 별도 parser module로 분리할 필요는 실제 구현 크기와
  diagnostics 복잡도를 보고 결정한다.
- parser generator나 lexer dependency는 현재 parser가 v0.2 acceptance를 막는다는
  재현 가능한 근거가 생길 때만 다시 평가한다.

## 구현 순서

1. file-aware source identity와 location diagnostics를 추가한다. (완료)
2. 여러 source file을 하나의 semantic/backend compilation unit으로 합친다. (완료)
3. multi-source compiler pipeline을 semantic, IR, C backend까지 연결한다. (완료)
4. manifest와 project discovery model을 구현한다.
5. 승인된 package/import/visibility token과 AST를 추가한다.
6. package별 declaration table과 import graph를 만든다.
7. cross-package semantic resolution과 visibility 검사를 연결한다.
8. project-aware `check`, `build`, `run`과 native smoke를 추가한다.

## 제외

- remote dependency와 package registry
- version constraint와 lockfile
- import alias, dot import, wildcard import
- package initialization hook
- interface와 dynamic dispatch

## 체크리스트

| ID | 상태 | 검증 | 작업 |
| --- | --- | --- | --- |
| C1 | pending | 사용자 decision | package/import/visibility surface 확정 |
| C2 | pending | 사용자 decision | manifest와 source layout 확정 |
| C3 | done | compiler/frontend tests | multi-file span, pipeline, cross-file error 구분 |
| C4 | pending | parser tests | package/import/`pub` syntax |
| C5 | pending | semantic rejection tests | unresolved import, visibility, cycle 진단 |
| C6 | pending | native project smoke | 두 package의 function/struct/method 호출 |

## 완료 기준

- `open-questions.md`의 language surface 질문이 닫혀 있다.
- 승인된 project model이 `SPEC.md`의 planned v0.2 section에 반영되어 있다.
- 구현 작업을 parser, project loader, semantic graph 단위로 나눌 수 있다.
