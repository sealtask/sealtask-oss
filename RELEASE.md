# Releasing SealTask OSS Crates

This workspace publishes to crates.io in dependency order:

1. `sealtask-client-core`
2. `sealtask-client-auth`
3. `sealtask-client-crypto`
4. `sealtask-client-api`
5. `sealtask-client-runtime`
6. `sealtask`

Downstream crates depend on earlier crates being visible on crates.io, so releasing them back-to-back without waiting will fail.

## Requirements

- a crates.io account with publish access
- `cargo login` already configured locally, or `CARGO_REGISTRY_TOKEN` set in the environment
- a clean git worktree unless you explicitly opt into `ALLOW_DIRTY=1`

For the 0.2.1 security maintenance release, require a clean `cargo audit`, the
full locked workspace suite, strict WASM manifest verification on Linux/AMD64,
and the browser StrongBox bridge integration test. Confirm that `wee_alloc` is
absent and that the release tag points at the same commit as the verified
artifact manifest.

For the 0.2.0 MFA compatibility release, confirm that every workspace package
and inter-crate dependency pin uses the same version. The new public
`PublicError::MfaRequiredUseBeginLogin` and `PublicError::MfaInputRequired`
variants can require additional arms in downstream exhaustive matches; keep the
dated compatibility note in README.md.

## Dry Run

From the repository root:

```bash
DRY_RUN=1 ./scripts/publish-crates.sh
```

Dry-run mode fully runs `cargo publish --dry-run` for `sealtask-client-core`, then packages downstream crates with `cargo package --no-verify --list`. That avoids crates.io index failures before the earlier internal crates are published.

## Publish

```bash
./scripts/publish-crates.sh
```

The script publishes each crate, then polls crates.io for the exact version before continuing to the next one.

After publishing, install `sealtask` in a clean environment and record
successful no-factor, interactive TOTP, one-time backup-code, one-line
`mfa_input_required`, and two-line stdin login flows. A path dependency or the
dry-run package result is not release evidence.

Also verify the installed CLI contract: `sealtask --json info` reports
`jsonContractVersion: 2`, `sealtask --json schema tasks create` is a single
compact JSON document, `--format json-pretty` is equivalent except for
whitespace, and two named profiles under a temporary `--config-dir` do not
share credentials or unlock state.

Generate the version-matched completion and manual bundle and attach it to
binary/package-manager releases:

```bash
SEALTASK_BINARY=target/release/sealtask ./scripts/generate-cli-assets.sh update
SEALTASK_BINARY=target/release/sealtask ./scripts/generate-cli-assets.sh check
find cli/assets -type f -print | sort
```

The bundle contains Bash, Zsh, Fish, and PowerShell completions plus section-1
manual pages for every visible command. Generate it from the exact release
binary; do not reuse assets from a prior CLI version.

Exercise data-key unlock against both a legacy version 1 account and an OPAQUE
export-key version 2 account. For version 2, record successful single-command,
unlock-daemon, and platform-keychain flows against the hosted API, including an
expired-access-token refresh. Confirm a wrong password leaves no bootstrap
secret and that credentials and local unlock storage contain no OPAQUE export
key. Version 1 password unwrap must remain offline.

For the two-line stdin check, confirm the first password line is still trimmed
and the second factor line loses only its physical LF or CRLF delimiter. Record
that whitespace-only, tabs, surrounding whitespace, Unicode digits, canonical
TOTP, and formatted or malformed backup-looking values reach the server
byte-for-byte; for an enrolled account, a missing or exactly empty line two must
produce `mfa_input_required`. Also confirm a denied factor is absent from
stderr/debug output and that no pending or final credential file is written.

## Useful Overrides

```bash
ALLOW_DIRTY=1 ./scripts/publish-crates.sh
WAIT_SECONDS=15 MAX_ATTEMPTS=40 ./scripts/publish-crates.sh
```

- `ALLOW_DIRTY=1`: allow packaging and publishing from a dirty worktree
- `WAIT_SECONDS`: seconds between crates.io visibility checks
- `MAX_ATTEMPTS`: maximum visibility checks before the script exits with an error

## Manual Recovery

If a publish succeeds but the script exits before the next crate, rerun it after the published version appears on crates.io. `cargo publish` will refuse to republish the same version, so you can continue safely after visibility catches up.
