# Spec: publish-finalizer-rc-gate

## Objective

- Ensure the approval-gated publish helper uses the full v0 RC verification
  gate before moving bookmarks or pushing remotes.
- Provide a no-push dry run for agents to validate the finalization path without
  requiring remote-publish approval.

## Scope

- Update `scripts/finalize-and-push.sh` to run `scripts/verify-v0-rc.sh`.
- Preserve the explicit approval boundary for real bookmark movement and remote
  push.
- Add `--no-push` for local verification of the finalization flow.
- Document the approval-gated command in the manifest, README, roadmap, and
  handoff.

## Checklist

- [x] Wire publish finalizer to `scripts/verify-v0-rc.sh`.
- [x] Add `--no-push` local dry-run path.
- [x] Keep real bookmark movement and remote push behind explicit invocation.
- [x] Record P87 in roadmap and handoff docs.

## Acceptance

| ID | Status | Command | Notes |
| --- | --- | --- | --- |
| C1 | done | `bash -n scripts/finalize-and-push.sh` | shell syntax gate |
| C2 | done | `scripts/finalize-and-push.sh --message "test: wire publish finalizer to rc gate" --no-push` | exercises description, v0 RC verification, no-push exit |
| C3 | done | `scripts/verify-v0-rc.sh --skip-deep-sanitizers` | fast RC gate still works after finalizer update |
