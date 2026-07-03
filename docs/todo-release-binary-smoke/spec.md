# Spec: release-binary-smoke

## Objective

- Verify the actual `target/release/mlg` binary before v0 publication.
- Keep release-candidate verification tied to the CLI artifact users would run,
  not only debug `cargo run` smokes.

## Scope

- Add `scripts/check-release-binary.sh`.
- Build `mlg` with `cargo build --release --bin mlg`.
- Verify release `--version` and `--help` output.
- Verify release `check examples/first.mlg`.
- Verify release `build examples/first.mlg` produces a runnable native binary.
- Wire the command into `scripts/verify-v0-rc.sh`.

## Checklist

- [x] Add release binary smoke script.
- [x] Add CLI version/help release smokes.
- [x] Add release binary check/build/run smokes.
- [x] Record P96 in roadmap and handoff docs.

## Acceptance

| ID | Status | Command | Notes |
| --- | --- | --- | --- |
| C1 | done | `scripts/check-release-binary.sh` | release binary smoke |
| C2 | done | `scripts/verify-v0-rc.sh --skip-deep-sanitizers` | v0 RC gate includes release binary smoke |
| C3 | done | `scripts/finalize-and-push.sh --message "test: publish v0 release candidate" --no-push` | non-publishing finalizer dry run remains green |
