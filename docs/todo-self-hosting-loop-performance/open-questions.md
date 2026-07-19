# Open Questions: Self-Hosting Loop Performance

Status: closed (2026-07-18)

## Should Fast Replace Full In The Canonical Gate?

No. The argument-free self-hosting command remains the complete milestone and
publication gate. Fast mode is an explicit local inner-loop profile and cannot
close a logical B2 change by itself.

## Which Coverage May Fast Mode Reduce?

Fast mode may reduce repeated execution configurations, not Stage0/Stage1
source coverage. Every discovered repository source still receives Rust/Mallang
differential comparison. The complete project-test suite and full-corpus
accounting/sanitizer matrix remain full-gate responsibilities.

## Which Optimization Levels Are Allowed?

Strict normal and accounting builds use `-O2`. Sanitizer builds use `-O1` with
ASan/UBSan and frame pointers. Any future change to these levels must repeat
oracle parity, accounting, sanitizer and full-gate timing evidence.

## Should Full-Gate Paths Run Concurrently?

Yes, for independent work with deterministic output ownership. Fixture and
parser-corpus jobs use separate result paths and run with a default cap of four.
The complete compiler-source link, prepare and check differentials also run as
independent background jobs. Each worker preserves exact oracle comparison,
stderr checks and failure status; sanitizer/profile variants of one fixture
remain inside the same worker so attribution stays local.

## What Is The Next Performance Boundary?

Generated test artifacts. The current test command emits and compiles the whole
project C translation unit once per selected test. Bounded `clang` concurrency
reduces wall time but does not remove roughly 1.76 GB of duplicate generated C.
A shared compiler object with small test harnesses, or one deterministic test
runner containing all selected tests, should precede lower-level incremental
compiler caching.
