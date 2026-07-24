# Spec: publish-remote-freshness-guard

Status: complete; historical milestone record

## Objective

- Prevent approval-gated publish from moving `main` when `origin/main` has
  advanced since the local stack was prepared.
- Keep non-publishing readiness commands side-effect-free with respect to
  bookmarks and remotes.

## Scope

- Add a real-publish-only freshness guard to `scripts/finalize-and-push.sh`.
- Fetch `origin` immediately before bookmark movement, preferring Homebrew Git
  when available so `jj git` has a new enough Git implementation.
- Compare the selected local bookmark base with the fetched remote bookmark.
- Abort publish with a clear diagnostic if the remote bookmark moved.
- Document the guard in README, handoff, release notes, and roadmap.

## Checklist

- [x] Add remote freshness guard before `jj bookmark set`.
- [x] Prefer Homebrew Git for `jj git fetch/push` when available.
- [x] Keep `--verify-only` and `--no-push` from running the remote push path.
- [x] Keep release helper contract checks green.
- [x] Record P91 in roadmap and handoff docs.

## Acceptance

| ID | Status | Command | Notes |
| --- | --- | --- | --- |
| C1 | done | `scripts/check-release-helpers.sh` | helper syntax/help contract includes freshness and Git fallback wiring |
| C2 | done | `scripts/verify-v0-rc.sh --skip-deep-sanitizers` | v0 RC gate remains green |
| C3 | done | `scripts/finalize-and-push.sh --verify-only` | full readiness gate remains non-publishing |
