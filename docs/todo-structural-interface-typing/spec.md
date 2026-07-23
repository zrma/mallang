# Structural Interface Typing

Status: deferred; nominal data remains stable while static structural interfaces await a post-v1 decision gate

## Problem

Mallang currently uses three different compatibility models:

- primitive, array, slice, `Option`, `Result`, and function types are compared by
  their complete type structure;
- named `struct` and `enum` declarations have nominal identity;
- interfaces, interface values, and dynamic dispatch do not exist in the v1
  language.

The language needs to decide whether future abstraction should follow
TypeScript-style structural record compatibility, Go-style structural interface
satisfaction, or an explicit implementation model. This decision intersects
with ownership modes, native layout, cleanup, receiver methods, package
visibility, generic specialization, and the stable v1 compatibility policy.

## Direction

The preferred direction is:

> Nominal data, structural behavior.

Named `struct` and `enum` declarations keep nominal identity. Two declarations
do not become assignable merely because their fields have the same names and
types. Their names continue to determine constructor identity, method scope,
generic specialization, native representation, and cleanup behavior.

A future named interface may instead be satisfied implicitly by a concrete
type's accessible method set. Source does not need an `implements` declaration.
The interface remains a named API contract, while conformance to that contract
is structural.

```mlg
type Named interface {
    func (con self) name() string
}
```

The example is directional syntax, not accepted v1 source. Exact declaration
and generic-constraint syntax remain part of the later decision gate.

## First Implementation Boundary

The first structural-interface slice should be compile-time only:

- interfaces describe methods, not fields or storage layout;
- conformance requires an exact method name and signature;
- receiver mode (`con`, `mut`, or owned), parameter modes, parameter types, and
  return type are all part of the required signature;
- generic specialization resolves interface-constrained operations statically;
- no value is implicitly converted to a runtime interface representation;
- no vtable, boxing, runtime type assertion, downcast, or reflection is added;
- no variance, optional method, overload, default method, blanket
  implementation, extension method, or associated type is added initially.

For cross-package conformance, only methods accessible through the existing
`pub` rules may satisfy a public interface use. A public interface signature
must not expose package-private types. Casing remains unrelated to visibility.

## Safety And Stability

Structural typing does not inherently weaken memory safety. A sound
implementation can preserve Mallang's ownership guarantees by matching every
ownership-relevant part of a method signature and specializing calls before
native lowering.

TypeScript-style structural compatibility for named records is not the target.
Mallang emits native layouts and type-specific cleanup behavior rather than
erasing object types into a garbage-collected runtime. Structural record
assignment would therefore require additional rules for:

- representation and field-order compatibility;
- cleanup and exactly-once ownership transfer;
- private fields and package boundaries;
- receiver method selection;
- API evolution when fields are added or removed.

Changing the identity or assignment rules of existing named `struct` and `enum`
types changes the v1 type and ownership contract and is a 2.0 boundary.

A compile-time-only structural interface can be considered for a compatible
1.x minor only if its syntax is contextual and unambiguous, every valid earlier
v1 program retains its behavior, and interface use introduces no implicit
runtime representation. First-class interface values and dynamic dispatch
require a separate design and compatibility review.

Once published, adding a required method to a public interface is a breaking API
change. Adding an unrelated method to a concrete type must not alter overload
selection or any existing program's behavior.

## Acceptance Boundaries

- Distinct named structs with identical fields remain non-assignable.
- Function types retain their existing complete-signature structural identity.
- Interface conformance diagnostics identify every missing or mismatched method
  component, including receiver and parameter modes.
- Generic interface calls monomorphize to concrete calls without runtime
  interface storage.
- `pub` visibility is enforced when computing cross-package method sets.
- Ownership, cleanup, Stage0/self-hosted differential, fixed-point, sanitizer,
  and native acceptance gates remain unchanged.
- Existing v1 source and observable semantics remain compatible.

## Deferred Work

- [x] Separate structural record compatibility from structural interface
  satisfaction.
- [x] Record nominal data and structural behavior as the preferred direction.
- [x] Preserve `pub` as the only visibility mechanism.
- [x] Bound the first candidate to static generic constraints.
- [ ] Choose contextual interface declaration and constraint syntax.
- [ ] Specify method-set lookup, interface composition, and diagnostics.
- [ ] Classify the first static-interface release as compatible 1.x or 2.0 from
  concrete parser and semantic examples.
- [ ] Design first-class interface values and dynamic dispatch separately, if a
  demonstrated use case requires them.
