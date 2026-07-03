# Spec: publish-post-push-verification

## Objective

- Treat publish success as remote bookmark equality, not only as a successful
  `jj git push` command.
- Fail clearly if the remote bookmark does not point at the commit the helper
  intended to publish.

## Scope

- Capture the publish target commit before moving the local bookmark.
- Push the selected bookmark through the existing Git-version fallback wrapper.
- Fetch the remote after push.
- Compare the fetched remote bookmark with the captured publish target.
- Keep `--verify-only` and `--no-push` non-publishing.

## Checklist

- [x] Capture publish target commit before `jj bookmark set`.
- [x] Add post-push remote bookmark verification.
- [x] Keep non-publishing paths out of bookmark movement and push.
- [x] Record P95 in roadmap and handoff docs.

## Acceptance

| ID | Status | Command | Notes |
| --- | --- | --- | --- |
| C1 | done | `scripts/check-release-helpers.sh` | helper contract verifies post-push wiring |
| C2 | done | `scripts/verify-v0-rc.sh --skip-deep-sanitizers` | v0 RC gate remains green |
| C3 | done | `scripts/finalize-and-push.sh --message "test: publish v0 release candidate" --no-push` | non-publishing finalizer dry run remains green |
