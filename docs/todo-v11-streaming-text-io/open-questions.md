# Mallang v1.1 Streaming Text I/O Decisions

Status: resolved for v1.1

## Q1: Reader value or callback?

Use one synchronous callback operation. A source-visible reader would conflict
with `V1-STD-002` and `V1-STD-008`, which keep native handles outside Mallang
v1. The runtime may own a handle internally for one call.

## Q2: How does caller state survive the callback?

Borrow generic `context` as `con C` and generic `state` as `mut S`. The callback
receives the same call-scoped borrows, so no closure capture, shared reference,
or owned accumulator transfer is required.

## Q3: What is a line?

LF terminates and is omitted. CR is ordinary preserved content. Embedded NUL is
preserved. Empty files have zero lines, blank lines are visited, and a terminal
LF does not add another empty line.

## Q4: Why v1.1.0?

The change adds a backward-compatible public standard-library API, so the
compatibility policy classifies it as a minor rather than patch release.
