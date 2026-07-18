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

Not in this slice. Serial normal, accounting and sanitizer execution keeps
failure attribution and supported-platform resource usage predictable. Bounded
parallelism may be evaluated later with explicit memory and CI-runner evidence.
