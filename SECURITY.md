# Security Policy

## Supported versions

Mallang 1.x is the supported stable release line. Pre-1.0 releases and release
candidates are retained for migration and provenance but do not receive routine
fixes.

## Reporting a vulnerability

Use GitHub private vulnerability reporting from the repository Security tab:

```text
https://github.com/zrma/mallang/security/advisories/new
```

Do not include exploit details, private environment information, credentials, or
other sensitive evidence in a public issue. Include the affected Mallang version,
platform, minimal reproduction, impact, and any relevant v1 rule identifiers in
the private report.

Security and memory-soundness fixes follow the narrow compatibility exception in
`docs/COMPATIBILITY.md`: the change must identify affected rules, add a regression
and migration path, and be called out in release notes.
