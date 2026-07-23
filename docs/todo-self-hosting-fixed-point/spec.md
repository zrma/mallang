# Spec: B4 Self-Hosting Fixed Point

Status: active; P178a complete, P178b-P178c pending

## Objective

B4 proves that the Mallang compiler built by Rust Stage0 reaches a stable
Mallang-owned compiler fixed point. Generated C, compiler-core behavior and
native safety evidence must agree before B5 may make the Mallang compiler the
default implementation.

## Work Breakdown

### P178a: Compiler Fixed Point

- build Stage1 from the declared compiler source set with Rust Stage0
- make Stage1 emit Stage2 C from the identical ordered source set and project
  graph
- compile Stage2 under strict C11 and make it emit the compiler again
- require Stage1-emitted and Stage2-emitted compiler C to be byte-identical
- compile the fixed compiler with ASan/UBSan, regenerate the compiler and
  require the same byte-identical output
- compare Stage1 and Stage2 status, stdout and stderr for every tracked lexer,
  parser, semantic, typed-IR, test-IR and flat backend fixture
- compare the complete repository parser corpus, compiler project operations
  and linked backend project C generation

The deep gate is `scripts/check-self-hosting-fixed-point.sh`. It is isolated
from `scripts/check.sh` so normal edit and canonical repository loops do not pay
the large self-compiler sanitizer cost. `--assume-bootstrap` and
`--skip-sanitizers` are diagnostic-only controls and never milestone evidence.

P178a is complete. The argument-free gate builds fresh Stage1 and strict-C11
Stage2, reaches a 9,780,069-byte compiler-C fixed point and regenerates the
identical output under ASan/UBSan. Stage1 and Stage2 also agree for 487
compiler-pair fixtures, the complete 168-source parser corpus, four compiler
project operations and eight linked backend projects. One development-host run
took 360 seconds; this is evidence for gate isolation, not a threshold.

### P178b: Complete Conformance Behavior

- compare every tracked package-layout, linker and standard-project operation
  between Stage1 and Stage2, including valid and rejection paths
- compile and run Stage1/Stage2-generated positive backend programs and require
  equivalent native output
- preserve B3 allocation-accounting, runtime-rejection and sanitizer behavior
  across the fixed-point compiler
- retain deterministic diagnostics for malformed and crash-corpus inputs

### P178c: B4 Closure

- run the argument-free fixed-point gate on every supported platform
- keep the deep job parallel to the canonical repository and release jobs
- pass publication boundary and private-inventory gates
- publish a logical `jj` change and require remote CI acceptance
- document the exact Stage0 seed retained for B5 and the default-transition
  rollback boundary

## Development Loop

1. Backend edits use
   `scripts/check-self-hosting-backend.sh --assume-bootstrap --fixtures-only`.
2. Fixed-point diagnostics may use
   `scripts/check-self-hosting-fixed-point.sh --assume-bootstrap --skip-sanitizers`.
3. A completed B4 slice runs the argument-free fixed-point gate.
4. Publication additionally runs `scripts/check.sh`, both publication gates and
   the supported-platform fixed-point CI matrix.

## Acceptance

- [x] Stage0 builds Stage1 from the declared source set
- [x] Stage1 builds strict-C11 Stage2 from the identical source set
- [x] Stage1 and Stage2 compiler C is byte-identical
- [x] sanitized Stage2 regenerates the identical fixed-point C
- [x] compiler-core fixture and repository parser-corpus behavior matches
- [x] compiler project and linked backend-project output matches
- [ ] complete project-graph and standard-project conformance matches
- [ ] native output, rejection, accounting and sanitizer behavior matches
- [ ] macOS arm64 and Linux x86_64 fixed-point CI acceptance
- [ ] B4 canonical publication acceptance

## Excluded

- deleting the Rust Stage0 bootstrap seed or differential oracle
- treating native executable bytes as deterministic compiler evidence
- moving the public default compiler from Rust Stage0 to Mallang before B5
- adding public syntax or standard APIs solely to shorten the bootstrap
