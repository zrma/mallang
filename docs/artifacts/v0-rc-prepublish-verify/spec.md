# Spec: v0-rc-prepublish-verify

Status: complete; historical milestone record

## Objective

- Add a single local v0 release-candidate verification command before remote
  publication.
- Keep the command non-publishing: it must not move bookmarks or push remotes.

## Scope

- Run the normal full gate through `scripts/check.sh`.
- Run the deep generated C sanitizer sweep against artifacts produced by the
  normal gate.
- Verify roadmap checkbox completion.
- Verify the local stack above `main` contains no empty changes.
- Verify Codex-authored local stack descriptions carry the configured
  attribution trailer.

## Checklist

- [x] Add `scripts/verify-v0-rc.sh`.
- [x] Add the command to repo manifest and README.
- [x] Record P86 in roadmap and handoff docs.
- [x] Keep remote publish as an explicit approval gate.

## Acceptance

| ID | Status | Command | Notes |
| --- | --- | --- | --- |
| C1 | done | `scripts/verify-v0-rc.sh` | normal gate, deep sanitizer gate, roadmap, stack, attribution |
| C2 | done | `scripts/verify-v0-rc.sh --skip-deep-sanitizers` | faster local rerun path |
| C3 | done | `bash -n scripts/verify-v0-rc.sh` | shell syntax gate |
