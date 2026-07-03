# Open Questions: publish-verify-only-finalizer

Remote publish still requires explicit user approval because
`scripts/finalize-and-push.sh --message "test: publish v0 release candidate"`
moves the selected bookmark and pushes it.
