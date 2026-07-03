# Spec: v0-rc-release-notes

## Objective

- Add a durable v0 release-candidate note that summarizes the local language
  surface, safety model, backend gates, verification command, deferred
  boundaries, and approval-gated publish command.

## Scope

- Add `docs/releases/v0-rc.md`.
- Link the release note from README and handoff docs.
- Record P88 in roadmaps.
- Do not move bookmarks or push remotes.

## Checklist

- [x] Add v0 RC release note.
- [x] Document verification and publish commands.
- [x] Record deferred post-v0 boundaries.
- [x] Keep publish approval-gated.

## Acceptance

| ID | Status | Command | Notes |
| --- | --- | --- | --- |
| C1 | done | `scripts/verify-v0-rc.sh --skip-deep-sanitizers` | local RC gate remains green |
| C2 | done | `scripts/check-release-helpers.sh` | release helper syntax remains valid |
