# Open Questions: B4 Self-Hosting Fixed Point

Status: active; P178a-P178b decisions frozen, P178c unresolved only by evidence

## Is Stage1-To-Stage2 Native Binary Identity Required?

No. Generated C is the deterministic compiler output. Linker versions, paths
and platform metadata make native executable bytes unsuitable fixed-point
evidence.

## Why Is The Fixed-Point Gate Separate From `scripts/check.sh`?

The 9 MB-class compiler C sanitizer build dominates the loop and does not
improve feedback for ordinary parser, semantic or backend edits. The deep gate
runs as a parallel supported-platform CI job and at completed B4 publication
boundaries. The canonical repository gate remains unchanged.

## May A Diagnostic Run Skip Sanitizers?

Yes, only with explicit `--skip-sanitizers`. Such a run can isolate C identity
or behavior mismatches but cannot satisfy P178 acceptance or publication.

## What Is The Declared Host Boundary?

The harness may sort source paths, pass the project graph and invoke `clang`.
Stage1 and Stage2 own parsing, linking, specialization, semantic and ownership
checking, typed IR and C generation. B5 owns migration of public project
discovery and native-build commands to the Mallang implementation.

## Do Unsupported Closures Block The Fixed Point?

No. The current compiler source does not use closures or intrinsic function
values. B4 proves the compiler source set and complete tracked compiler-core
behavior; general backend closure support remains a separate language feature
boundary.
