# Spec: v0.5-ownership-runtime

상태: proposed, approval required

## 목표

- v1 후보 type 전체의 Copy, move, borrow와 drop 규칙을 하나의 memory model로 닫는다.
- Raw pointer나 user-visible lifetime 없이 owned recursive data를 표현한다.
- Normal control flow의 owned temporary와 aggregate replacement를 deterministic cleanup에
  연결한다.
- Fatal runtime failure와 recoverable `Result` 경계를 분리한다.

## 승인 대기 결정

- Value classification: primitive와 built-in ADT의 현재 Copy 경계를 유지하고 나머지
  aggregate/resource type은 move-only로 둔다.
- String storage: static literal과 heap-backed immutable text를 하나의 move-only
  runtime representation으로 통합한다.
- Recursive ownership: predeclared `Heap[T]`, `heap(value)`, `unheap(value)`를 사용한다.
- Field extraction: uninitialized partial move 대신
  `replace(mut place, replacement) -> T`를 사용한다.
- Temporary: full-expression lifetime과 deterministic normal-flow drop을 구현한다.
- Borrow: `con`/`mut`는 call-scoped로 유지하고 first-class reference와 by-reference
  range를 v1 후보에서 제외한다.
- Failure: fatal runtime error는 stderr + non-zero no-unwind termination이며 expected
  failure는 v0.6 `Result` API로 넘긴다.

## Memory Model

| Type | Copy | Runtime resource | Drop behavior |
| --- | --- | --- | --- |
| `int`, `bool`, `unit` | yes | none | no-op |
| `Option[T]` | when `T` is Copy | payload-dependent | active payload drop |
| `Result[T, E]` | when both payloads are Copy | payload-dependent | active payload drop |
| `string` | no | static or owned text storage | free owned storage only |
| `[N]T` | no | element-dependent | drop active elements |
| `[]T` | no | owned buffer | drop elements, then free buffer |
| user struct | no | field-dependent | drop fields in reverse declaration order |
| user enum | no | active payload-dependent | drop active payload |
| function value/closure | no | optional owned environment | drop captures, then free environment |
| `Heap[T]` | no | one non-null allocation | drop payload, then free allocation |

User-defined copy derivation, destructor and finalizer는 v0.5 범위가 아니다.

## Heap Surface

```mlg
next := heap(List[int].Nil)
node := Node[int]{head: 1, tail: next}
owned := heap(node)
show(con owned.value)
replace(mut owned.value.head, 2)
nodeAgain := unheap(owned)
```

- `Heap[T]`는 exactly one owner를 가진다.
- `heap`은 payload ownership을 consume한다.
- `unheap`은 handle ownership을 consume하고 payload ownership을 반환한다.
- `.value`는 place projection이며 owned move source가 아니다.
- Recursive type validation은 `Heap` edge에서 by-value cycle 탐색을 멈춘다.
- Heap allocation은 checked size와 null failure guard를 사용한다.

## Replace Surface

```mlg
old := replace(mut target.field, replacement)
```

- Target는 mutable local-rooted place 또는 `Heap.value` projection이어야 한다.
- Target와 replacement type은 같아야 한다.
- Target base/index는 한 번만 평가한다.
- Replacement는 old value를 꺼내기 전에 temporary로 완전히 평가한다.
- Old value는 결과로 move되고 target는 replacement로 즉시 다시 초기화된다.
- Result를 버리면 full-expression cleanup이 old value를 drop한다.
- 일반 uninitialized partial move와 runtime field drop flag는 만들지 않는다.

## Temporary and Cleanup Rules

Normal control flow에서 owned value마다 정확히 한 번 다음 중 하나가 일어난다.

1. 다른 owner로 move한다.
2. `Copy` value로 복사한다.
3. 마지막 정상-flow lifetime 끝에서 drop한다.

Cleanup scope:

- Statement expression temporary는 full-expression 끝이다.
- Call argument temporary는 call이 반환된 뒤다.
- Condition temporary는 condition 평가가 끝난 뒤다.
- Branch-local temporary는 선택된 branch를 떠날 때다.
- Loop source temporary는 loop exit, `break` 또는 enclosing function exit에 맞는
  cleanup path를 가진다.
- Return expression은 return temporary로 먼저 평가하고 callee local cleanup 뒤
  caller에게 ownership을 전달한다.
- Overwrite는 right-hand side 평가 성공 뒤 old destination을 drop하고 새 owner를
  저장한다.
- Fatal runtime termination은 unwind하지 않으므로 이 normal-flow 규칙 밖이다.

## Borrow Rules

- `con expr`와 `mut expr`는 direct call argument mode다.
- Borrow는 callee return 시 끝난다.
- Borrowed non-Copy value는 move, return, store 또는 capture할 수 없다.
- 같은 call 안에서 shared/shared는 허용하고 shared/exclusive 및 overlapping
  exclusive borrow는 거부한다.
- Field/index place disjointness는 compiler가 증명할 수 있는 범위만 허용한다.
- First-class reference, lifetime annotation, borrowed return과 borrowed collection view는
  v0.5/v1 후보 surface에 포함하지 않는다.
- Range element mutation은 indexed place assignment/replace/call borrow로 표현한다.

## Runtime Failure Contract

- Fatal guard는 stderr에 안정적인 Mallang runtime error를 쓰고 non-zero로 종료한다.
- Fatal failure는 catch할 수 없고 stack unwinding이나 user cleanup을 실행하지 않는다.
- Allocation size overflow와 allocation failure는 unchecked C behavior 전에 guard한다.
- Normal program exit은 모든 compiler-owned allocation을 정리한다.
- Recoverable I/O, parse와 environment failure는 v0.6 library에서 `Result`로 전달한다.

## 구현 순서

1. Q1-Q7 추천안을 승인받고 이 문서를 확정한다.
2. Compiler-wide value/drop classification API와 type matrix regression을 추가한다.
3. Full-expression temporary와 branch/loop/return cleanup normalization을 완성한다.
4. `replace` place semantics와 overwrite/cleanup lowering을 추가한다.
5. `Heap[T]`, `heap`, `unheap`, recursive type validation과 native layout/drop을 추가한다.
6. `string` runtime representation을 static/owned storage로 일반화한다.
7. Borrow/range exclusion diagnostic과 SPEC memory model을 동기화한다.
8. Allocation accounting, failure injection, strict C와 sanitizer acceptance를 연결한다.

## 제외

- Raw pointer, address-of와 pointer arithmetic
- Null/nullable `Heap`
- Shared ownership, reference counting와 cycle collector
- First-class reference와 user-visible lifetime
- By-reference/mutable range binding
- General uninitialized partial move와 runtime field drop flag
- User-defined destructor/finalizer
- Panic/recover, exception와 stack unwinding
- Recoverable standard-library error API 구현

## 완료 기준

- Recursive `Heap` data가 native compile/run되고 non-Copy payload를 exactly once drop한다.
- Full-expression temporary가 call, condition, index, range와 discarded expression에서
  leak이나 double-drop 없이 동작한다.
- `replace`가 local/field/index/heap place에서 old ownership을 안전하게 반환한다.
- String literal과 heap-backed string의 move/drop contract가 동일 type 아래 검증된다.
- Known use-after-move, borrowed escape, overlap, partial move와 cleanup merge 오류를
  source diagnostic으로 거부한다.
- Return, branch, loop, early exit와 overwrite cleanup이 typed IR regression으로
  고정된다.
- Cleanup-heavy generated C가 strict warning-clean, ASan/UBSan과 normal-exit allocation
  accounting을 통과한다.
- `SPEC.md`, implementation과 runtime failure behavior가 같은 memory model을 설명한다.
