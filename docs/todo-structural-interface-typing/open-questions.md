# Open Questions

## Q1. What source syntax introduces an interface?

The leading shape is a contextual `type Name interface { ... }` declaration so
`interface` does not become a globally reserved identifier in 1.x. The parser
grammar and formatter need concrete before/after examples before this is
approved.

## Q2. How are generic constraints written?

The constraint syntax should extend the existing `func Name[T]` and generic type
forms without introducing ownership ambiguity. Candidate syntax and diagnostics
remain intentionally undecided.

## Q3. Which methods satisfy a cross-package interface?

The preferred rule is that only methods accessible at the use site count.
Public-interface signatures and satisfying public methods must not expose
package-private types. Same-package private interfaces may inspect the local
method set.

## Q4. Are multiple constraints or interface composition included initially?

No. The first slice should prove one named interface constraint and exact method
matching. Composition, intersections, associated types, and default methods need
independent use cases.

## Q5. When are runtime interface values introduced?

Not with static structural conformance. Existential values, dynamic dispatch,
boxing, vtables, ownership erasure, and downcasting form a separate future
decision gate.

## Q6. Which release line may contain the first static interface?

A compatible 1.x minor is possible only for contextual additive syntax with no
change to existing type identity or observable semantics. Otherwise the feature
waits for 2.0. The release class is decided from executable compatibility
examples before implementation.
