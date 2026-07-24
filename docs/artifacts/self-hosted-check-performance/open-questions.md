# Open Questions

## Q1. Is incremental compilation required for the first target?

No. The measured clean-check hotspot is repeated UTF-8 validation. Fix the
algorithmic full-check cost before introducing cache invalidation and persistent
artifact complexity.

## Q2. Does layout-only equality weaken source-level UTF-8 safety?

No for valid Mallang programs. Source literals and external byte ingress validate
UTF-8 before values enter the language string invariant, and owned-string
operations preserve that invariant. Equality does not interpret scalar
boundaries; it compares length and bytes. It still validates storage kind, data
pointer, and length before reading memory.

## Q3. Is the one-second target a compatibility guarantee?

No. It is a local milestone against the recorded reference input and machine.
The durable regression protection is the generated-helper contract plus
correctness gates; wall time remains observational until supported-platform
variance supports a portable threshold.
