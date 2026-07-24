# P158 Machine-readable Diagnostics

상태: complete

## Goal

기존 human diagnostic을 유지하면서 compiler, CLI와 native runner가 같은 구조화 진단
모델을 사용하게 한다. Editor와 자동화는 versioned JSON Lines를 소비하고, 이후
multi-error recovery는 기존 record를 바꾸지 않고 record 수를 늘리는 방향으로 확장한다.

## CLI Contract

Diagnostic format은 subcommand 앞의 global option이다.

```text
mlg [--diagnostic-format <human|json>] <subcommand> ...
```

- 기본값은 `human`이다.
- `--diagnostic-format json`과 `--diagnostic-format=json`을 지원한다.
- Option은 subcommand 앞에만 둔다. Subcommand별 option으로 중복 정의하지 않는다.
- 성공한 명령의 stdout과 exit status는 format에 따라 바뀌지 않는다.
- JSON mode의 compiler-owned diagnostic은 stderr에 record 하나당 한 줄로 출력한다.
- 알 수 없는 format은 mode를 선택할 수 없으므로 human CLI diagnostic으로 거부한다.

## Schema v1

Schema identifier는 `mallang.diagnostic.v1`이다.

```json
{"schema":"mallang.diagnostic.v1","severity":"error","stage":"semantic","message":"use of moved value `value`","source":{"path":"src/main.mlg","span":{"byte_start":10,"byte_end":15,"start":{"line":2,"column":5},"end":{"line":2,"column":10}}}}
```

Top-level field contract:

- `schema`: exact string `mallang.diagnostic.v1`
- `severity`: v1에서는 `error`
- `stage`: `cli`, `input`, `frontend`, `package`, `link`, `semantic`, `ir`,
  `backend`, `native` 중 하나
- `message`: 위치 prefix나 human diagnostic 전체를 포함하지 않는 진단 본문
- `source`: source를 특정할 수 있을 때만 존재하는 object
- `source.path`: UTF-8 JSON string으로 표시한 source path
- `source.span`: exact byte와 line/column 위치를 모두 계산할 수 있을 때만 존재

Span contract:

- `byte_start`와 `byte_end`는 source UTF-8 byte 기준의 start-inclusive,
  end-exclusive offset이다.
- `start`와 `end`의 `line`/`column`은 1-based Unicode scalar 기준이다.
- Source가 있지만 span이 없는 input/native error는 `source.path`만 기록할 수 있다.
- Source를 특정할 수 없는 CLI 또는 graph error는 `source`를 생략한다.

## Stage Ownership

| stage | owner |
| --- | --- |
| `cli` | global/subcommand argument parsing과 usage |
| `input` | source/project discovery, file I/O와 format 상태 |
| `frontend` | lexing, parsing과 formatter syntax validation |
| `package` | package declaration/import graph construction |
| `link` | package symbol resolution과 visibility linking |
| `semantic` | type, ownership와 language semantic checks |
| `ir` | checked program to typed IR lowering |
| `backend` | typed IR to generated C lowering |
| `native` | C compiler invocation, test child와 native execution boundary |

## Path Contract

- Standalone source는 caller가 넘긴 display path를 유지한다.
- Root project source와 test는 project root 기준 `src/...` 또는 `tests/...`를 사용한다.
- Local dependency source는 `<dependency-project>/src/...`처럼 project name을 prefix로
  사용한다. Dependency가 root 아래에 있거나 sibling directory에 있어도 같은 표기다.
- Formatter project diagnostics는 root-relative path를 사용한다.
- JSON path는 URI가 아니며 v1에서 canonical filesystem path를 노출하는 API가 아니다.

## Rendering And Streams

Human과 JSON renderer는 같은 `Diagnostic` value를 입력으로 사용한다. Human form은
span이 있으면 `path:line:column: message`, path만 있으면 `path: message`, source가 없으면
`message`를 유지한다. Formatter check는 변경 파일별 record를 출력하므로 JSON mode에서
여러 line이 될 수 있다.

JSON string 안의 newline은 JSON escaping을 사용하므로 각 record는 물리적으로 한 줄이다.
이 계약은 compiler-owned diagnostic에 적용된다. `mlg run` 대상 프로그램이나 실패한 test
child가 직접 쓴 stdout/stderr는 프로그램 출력이며 diagnostic으로 감싸지 않는다. Editor
consumer는 실행 출력을 섞지 않는 `mlg check`를 사용한다.

## Consumer And LSP Review

`tests/fixtures/diagnostics/consume-jsonl.py`는 standard-library-only JSONL consumer다.
Schema, stage, source/span shape와 단위를 검증하고 같은 record에서 human form을 재구성한다.
`scripts/check-diagnostics.sh`는 CLI/input/frontend/package/link/semantic/native 대표 오류,
formatter multi-record, failed test assertion, human parity와 successful-command stdout 불변성을
debug/release binary에서 검증한다. `ir`/`backend` spelling은 의도적인 internal invariant
failure를 만들지 않고 library unit regression으로 고정한다.

P158 evidence로 one-shot editor consumer는 가능하지만 parser recovery, multiple diagnostics,
document overlay, incremental project state와 cancellation은 여전히 없다. 따라서 stdio LSP와
editor packaging은 v0.7 blocker가 아니며 P160의 v0.8 decision gate까지 보류한다.

## Acceptance

- [x] versioned serializable diagnostic model과 stable stage vocabulary
- [x] human/JSON shared renderer와 existing human CLI parity
- [x] UTF-8 byte span 및 1-based Unicode scalar location regression
- [x] global JSON mode와 stderr JSON Lines contract
- [x] standalone/project/dependency diagnostic path normalization
- [x] formatter multi-record와 test/native diagnostic integration
- [x] JSONL consumer fixture와 debug/release binary smoke
- [x] basic LSP release-blocker 재평가

## Excluded

- Diagnostic code, warning/note/help severity와 related spans
- Parser recovery와 multi-error compiler collection
- JSON-RPC/LSP server, document overlay와 incremental compilation
- Editor-specific plugin과 protocol packaging
