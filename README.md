# SealTask OSS

Open-source Rust workspace for the `worklist` CLI and shared client crates.

This repository contains the early public client surface for SealTask:

- `worklist`: command-line client for authenticating, reading decrypted work lists/tasks/comments, and creating or updating tasks and comments
- `worklist-client-core`: shared public types and error handling
- `worklist-client-auth`: local credential storage and authentication helpers
- `worklist-client-api`: typed HTTP client for the SealTask API
- `worklist-client-crypto`: client-side crypto helpers for sealed payloads and key derivation
- `worklist-client-runtime`: unlock-aware runtime that projects raw API responses into agent-facing decrypted models

## Status

This workspace is still in active development and is not yet positioned as a stable public SDK.

- crate boundaries are intentional, but APIs may still change
- several APIs may still evolve as the agent workflow surface expands
- the current release target is the CLI first, with supporting crates published alongside it

## 0.2.0 compatibility note (2026-07-12)

Version 0.2.0 adds the MFA-aware `begin_login` / `complete_mfa_login` flow and
the public `PublicError::MfaRequiredUseBeginLogin` and
`PublicError::MfaInputRequired` variants. Downstream code that exhaustively
matches `PublicError` must add arms for both variants. The existing `login`
function remains available for accounts without MFA; for an enrolled account it
wipes the pending challenge and returns the typed upgrade-to-begin error instead
of exposing or persisting challenge state.

For `worklist auth login --password-stdin`, line one is the password and an
optional line two is the TOTP or backup code. The first line keeps its existing
trimmed-password behavior. On line two, the physical LF or CRLF record delimiter
is removed and every other byte is preserved: spaces, tabs, leading or trailing
whitespace, Unicode, and a lone carriage return are sent to the server unchanged.
A missing or exactly empty second line means no factor was supplied. Extra lines
containing non-whitespace are rejected. An enrolled account given only line one
returns `mfa_input_required` and never prints the raw challenge or falls back to
a terminal prompt. Other `--password-stdin` commands keep their existing
whole-input password contract.

The CLI accepts both data-key wrapper formats used by SealTask accounts. Legacy
version 1 wrappers are unwrapped locally from the password. Version 2 wrappers
reacquire the account's OPAQUE export key through the authenticated
`/auth/opaque/export-key/start` challenge, derive the wrapping key locally, and
discard the export key after the command. A password-based version 2 unlock
therefore requires network access and a valid access or refresh session. The
unlock daemon and platform-keychain bootstrap continue to work offline once
they already hold the decrypted data key; the OPAQUE export key is never stored
in credentials, the daemon, or the platform keychain.

Because version 2 password unwrap performs an authenticated network request,
`RuntimeClient::unlock_daemon` and `RuntimeClient::store_persisted_data_key` are
now asynchronous and must be awaited by downstream callers.

## Layout

```text
cli/                    # public CLI binary
crates/client-core/     # shared public types and errors
crates/client-auth/     # auth, credentials, and session helpers
crates/client-api/      # typed API client
crates/client-crypto/   # client-side crypto and payload helpers
crates/client-runtime/  # decrypted agent-facing runtime and read models
.github/workflows/ci.yml
```

## Getting Started

Requirements:

- Rust stable toolchain

Common commands:

```bash
cargo check
cargo test
cargo run -p worklist -- --help
cargo run -p worklist -- auth unlock --password-stdin
cargo run -p worklist -- auth keychain store --password-stdin
cargo run -p worklist -- --json tasks get --work-list-id <list-id> --task-id <task-id>
cargo run -p worklist -- --json tasks attachments read --work-list-id <list-id> --task-id <task-id> --attachment-id <attachment-id>
cargo run -p worklist -- --json tasks attachments download --work-list-id <list-id> --task-id <task-id> --attachment-id <attachment-id>
```

Example enrolled-account automation input:

```bash
printf '%s\n%s\n' "$SEALTASK_PASSWORD" "$SEALTASK_MFA_CODE" \
  | cargo run -p worklist -- auth login --email user@example.com --password-stdin
```

Do not pass authenticator or backup codes as command-line arguments; arguments
can be retained in shell history and process listings.

Once the crate is published, install the CLI with:

```bash
cargo install worklist
```

Set a custom API URL with `WORKLIST_API_URL` if you are not targeting the default hosted endpoint:

```bash
WORKLIST_API_URL=https://your-worklist.example cargo run -p worklist -- me
```

## Development Notes

- The CLI defaults to table/text output for humans; pass `--json` for machine-readable output.
- Read commands return decrypted agent-facing models by default; raw wire DTOs are only available through hidden debug flags.
- Encrypted read and write commands are non-interactive by default. Use `auth unlock --password-stdin` for a temporary in-memory session, or `auth keychain store --password-stdin` to persist a local bootstrap secret in the platform keychain.
- Password unlock supports legacy password-wrapped version 1 accounts and OPAQUE export-key version 2 accounts. Version 2 password unlock contacts the authenticated API; later daemon- or keychain-backed commands do not repeat that exchange.
- `tasks get` includes typed attachment metadata and lists attachment IDs in table output.
- `tasks attachments read` prints readable attachments to stdout, including plain text passthrough and DOCX rendered as Markdown; with `--json` it emits the rendered content plus attachment metadata.
- `tasks attachments download` decrypts binary attachments and saves them locally; if `--output` is omitted it writes `./<attachment-file-name>`.
- The current workspace targets encrypted SealTask flows, so authenticated reads and writes still depend on credentials, local key unwrap, and workspace keys from a live SealTask deployment.
- CI for this repository runs from `.github/workflows/ci.yml`.
- Crates.io release steps are documented in [`RELEASE.md`](./RELEASE.md), with a helper script at [`scripts/publish-crates.sh`](./scripts/publish-crates.sh).

## Repository Flow

This public repository is mirrored automatically from SealTask's upstream development repository. The code here is intended to be consumable as a normal standalone Rust workspace, but some changes may land here after first being developed upstream.

## License

This workspace is licensed under `GPL-3.0-only`. See [LICENSE](./LICENSE).
