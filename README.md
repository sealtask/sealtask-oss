# SealTask OSS

Open-source Rust workspace for the `sealtask` CLI, shared client crates, and
the browser StrongBox WASM engine. The canonical repository is
[`sealtask/sealtask-oss`](https://github.com/sealtask/sealtask-oss).

This repository contains the early public client surface for SealTask:

- `sealtask`: command-line client for authenticating and working with decrypted work lists, tasks, comments, notes, and task attachments
- `sealtask-client-core`: shared public types and error handling
- `sealtask-client-auth`: local credential storage and authentication helpers
- `sealtask-client-api`: typed HTTP client for the SealTask API
- `sealtask-client-crypto`: client-side crypto helpers for sealed payloads and key derivation
- `sealtask-client-runtime`: unlock-aware runtime that projects raw API responses into agent-facing decrypted models
- `strong-box`: SealTask's GPL-3.0 StrongBox fork used by the browser engine
- `strong-box-wasm`: the Rust-to-WASM bindings shipped in the SealTask browser client

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

For `sealtask auth login --password-stdin`, line one is the password and an
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
crates/strong-box/       # StrongBox fork used by the browser WASM build
crates/strong-box-wasm/  # browser WASM ABI and cryptographic bindings
artifacts/strong-box-wasm/
                        # canonical WASM byte and strict build manifest
scripts/build-strong-box-wasm.sh
                        # pinned build/update/verification entrypoint
.github/workflows/ci.yml
```

## Getting Started

Requirements:

- Rust 1.94.0 (also pinned by `rust-toolchain.toml`)
- Python 3.11 or newer for strict manifest generation and verification

Common commands:

```bash
cargo check --workspace --all-targets --locked
cargo test --workspace --all-targets --locked
./scripts/build-strong-box-wasm.sh build
./scripts/build-strong-box-wasm.sh verify
cargo run -p sealtask -- --help
cargo run -p sealtask -- auth unlock --password-stdin
cargo run -p sealtask -- auth keychain store --password-stdin
cargo run -p sealtask -- --json tasks get --work-list-id <list-id> --task-id <task-id>
cargo run -p sealtask -- --json tasks attachments read --work-list-id <list-id> --task-id <task-id> --attachment-id <attachment-id>
cargo run -p sealtask -- --json tasks attachments download --work-list-id <list-id> --task-id <task-id> --attachment-id <attachment-id>
cargo run -p sealtask -- --json notes list --work-list-id <list-id>
```

## Browser WASM provenance

`crates/strong-box` and `crates/strong-box-wasm` are the production source for
the WASM byte shipped by SealTask. All development, CI, and Docker builds use
Rust 1.94.0, the `wasm32-unknown-unknown` target, Cargo's `wasm-release`
profile, the checked-in lockfile, and the same path-remapped build script.
Unpinned, host-dependent `wasm-opt` post-processing is deliberately not used.

The canonical Linux/AMD64 byte and its strict manifest live in
`artifacts/strong-box-wasm/`. The verifier rebuilds from this workspace and
requires byte-for-byte equality with the artifact, its SHA-256 and size in the
manifest, and the manifest's Cargo lockfile digest and toolchain metadata.

Maintainers refresh the checked artifact only on Linux/AMD64:

```bash
./scripts/build-strong-box-wasm.sh update
git diff -- artifacts/strong-box-wasm/
./scripts/build-strong-box-wasm.sh verify
```

An immutable release tag binds the manifest and artifact to the public source
tree that contains them.

## Agent task automation

The supported automation mode acts through an authenticated user session. Log
in once, then either unlock the local daemon for a bounded session or store the
decrypted data-key bootstrap in the platform keychain:

```bash
printf '%s\n' "$SEALTASK_PASSWORD" \
  | cargo run -p sealtask -- --json --non-interactive auth login \
      --email agent-user@example.com --password-stdin

printf '%s\n' "$SEALTASK_PASSWORD" \
  | cargo run -p sealtask -- --json --non-interactive auth unlock \
      --ttl-seconds 28800 --password-stdin
```

Create a task with the native lifecycle fields and a retry key:

```bash
cargo run -p sealtask -- --json --non-interactive tasks create \
  --work-list-id <list-id> \
  --title 'Prepare release notes' \
  --priority 5 \
  --start-at 2026-08-09T08:00:00Z \
  --due-at 2026-08-10T09:30:00Z \
  --section-id <section-id> \
  --idempotency-key 'agent:run-42:release-notes'
```

Priorities are `1`, `3`, `5`, or `8`; date arguments are RFC 3339 timestamps.
An idempotency key is scoped to the user and should remain stable for retries of
one logical create. A retry with the same readable task semantics returns the
original task even though encryption uses a fresh nonce. Reusing the key for
different semantics, or retrying after the original task was deleted, returns a
conflict instead of creating another task.

Checklist create and update operations use structured JSON. Task-level fields
are camelCase; checklist payload fields retain their encrypted payload schema's
snake_case names:

```json
{
  "title": "Prepare release notes",
  "body": "Summarize user-visible changes.",
  "checklist": [
    {
      "id": "019f0000-0000-7000-8000-000000000001",
      "title": "Collect merged changes",
      "is_done": false,
      "assignee_user_ids": []
    }
  ],
  "priority": 5,
  "startAt": "2026-08-09T08:00:00Z",
  "dueAt": "2026-08-10T09:30:00Z",
  "sectionId": "019f0000-0000-7000-8000-000000000002",
  "idempotencyKey": "agent:run-42:release-notes"
}
```

Pass this document with `--input-file <path>`. `--input-stdin` is also
supported, but cannot share stdin with `--password-stdin`; use an unlocked
daemon/keychain or an input file when both task JSON and a password are needed.

Structured updates use patch semantics: an omitted field is unchanged, `null`
clears a nullable field, and a value sets it. The equivalent flag form provides
`--clear-body`, `--clear-priority`, `--clear-due-at`, `--clear-start-at`, and
`--clear-section`. Every update and move is sent with the revision just read by
the CLI; a concurrent change returns a conflict so the agent can re-read,
reconcile, and retry instead of silently overwriting newer state.

```bash
cargo run -p sealtask -- --json --non-interactive tasks complete \
  --work-list-id <list-id> --task-id <task-id>
cargo run -p sealtask -- --json --non-interactive tasks reopen \
  --work-list-id <list-id> --task-id <task-id>
```

SealTask's board model defines completion by section: `complete` moves the task
to the final section and `reopen` moves it to the first section. These commands
require a work list with at least two sections and are idempotent when the task
already has the requested state.

### Notes

Notes support decrypted list/get plus encrypted create/update/delete. Shared notes
use the work-list key; `--private` creates a per-note key that is wrapped with
the current user's data key.

```bash
cargo run -p sealtask -- --json --non-interactive notes create \
  --work-list-id <list-id> \
  --title 'Release context' \
  --body 'Keep this with the project.' \
  --idempotency-key 'agent:run-42:release-context'

cargo run -p sealtask -- --json --non-interactive notes create \
  --work-list-id <list-id> \
  --title 'Private scratchpad' \
  --private \
  --idempotency-key 'agent:run-42:private-scratchpad'

cargo run -p sealtask -- --json --non-interactive notes update \
  --work-list-id <list-id> --note-id <note-id> \
  --title 'Revised title' --body 'Revised body'
```

Create and update also accept camelCase JSON through `--input-file` or
`--input-stdin`. Updates preserve encrypted mentions, attachment references,
and client metadata that are not exposed as editing flags.

Note creation requires an `idempotencyKey` (or `--idempotency-key`) that stays
stable for every retry of that logical note. SealTask commits the user-scoped
key with an opaque work-list-keyed semantic commitment, so a retry returns the
original note even though encryption uses a fresh nonce. Reusing a key for
different note content, privacy, project, or creator returns a conflict. Keep
the key in the automation state before invoking the CLI; this is what makes a
retry safe after Ctrl-C, a lost response, or a process crash.

SDK callers that use `sealtask-client-api` directly must use its documented
`note_transport` module. Note HTTP methods accept and return sealed,
model-specific `EncodedNoteRequest<T>` and `EncodedNoteResponse<T>` wrappers;
their synchronous codecs are intended to run on a caller-owned bounded blocking
executor. `sealtask-client-runtime` supplies that admission automatically and
never performs maximum-size note JSON work on a Tokio worker.

### Task attachment uploads and deletion

```bash
cargo run -p sealtask -- --json --non-interactive tasks attachments upload \
  --work-list-id <list-id> --task-id <task-id> \
  --file ./release-notes.md

cargo run -p sealtask -- --json --non-interactive tasks attachments delete \
  --work-list-id <list-id> --task-id <task-id> \
  --attachment-id <attachment-id>
```

Upload encrypts file bytes locally with a fresh key, uploads only ciphertext,
then stores the wrapped file reference inside the encrypted task payload. Files
must be non-empty and fit within the API's 10 MiB ciphertext limit. `--file-name`
and `--content-type` override the inferred metadata. Delete removes the
encrypted task reference and attachment ID together; the API then removes the
orphaned attachment and permanently seals its storage key against replay.

Upload paths are resolved only beneath the CLI's already-open current working
directory. They must be relative, must name regular files directly, and cannot
contain parent traversal; absolute paths and symbolic links (including
intermediate directory links) are rejected.
Pressing Ctrl-C is observed before the upload and between its bounded
side-effecting stages. A storage PUT that has already started is awaited to its
bounded result before the CLI makes a bounded attempt to release the
server-side reservation.

Presigned upload and download capabilities are accepted only for explicitly
trusted storage origins. The API origin is trusted automatically. If your
deployment signs URLs on another origin, repeat `--storage-origin` or set a
comma-separated `SEALTASK_STORAGE_ORIGINS` value:

```bash
SEALTASK_STORAGE_ORIGINS=https://objects.example \
  cargo run -p sealtask -- tasks attachments upload \
    --work-list-id <list-id> --task-id <task-id> --file ./release-notes.md
```

Storage transfers require HTTPS, reject redirects and unsafe network targets,
and use deadlines capped by the signed URL's expiry. Loopback HTTP storage is
enabled only when the configured API URL is itself loopback HTTP, for local
development and tests.

### JSON process contract

`sealtask --json info` reports `"jsonContractVersion": 1`. For ordinary commands run
with `--json --non-interactive`, version 1 guarantees:

- `--json` emits one compact JSON document; `--format json-pretty` emits the
  same document with indentation
- success writes exactly one JSON document to stdout; stderr is empty unless a
  structured warning is emitted
- collection commands always write a JSON array, including `[]` for an empty
  result
- warnings and runtime errors are written to stderr as one JSON envelope, for
  example `{"warnings":[...]}` or
  `{"error":{"code":"validation","message":"...","retryable":false}}`
- errors may also expose `retryAfterSeconds`, `backendCode`, `httpStatus`,
  `outcome`, and an actionable `hint`; agents should branch on fields rather
  than message copy
- successful commands exit `0`, runtime/validation failures exit `1`, and Clap
  argument-parsing failures retain Clap's exit code (`2` for usage errors)
- a closed stdout pipe is treated as successful consumer termination and exits
  `0`

Help and version output retain Clap's human-readable text format. Use
`sealtask --json schema [COMMAND ...]` for a versioned machine-readable command
and argument description.

`--json` controls presentation; `--non-interactive` controls prompts. Automation
should pass both. Human sessions can request pretty JSON while retaining
interactive auth prompts.

Example enrolled-account automation input:

```bash
printf '%s\n%s\n' "$SEALTASK_PASSWORD" "$SEALTASK_MFA_CODE" \
  | cargo run -p sealtask -- --json --non-interactive auth login \
      --email user@example.com --password-stdin
```

Do not pass authenticator or backup codes as command-line arguments; arguments
can be retained in shell history and process listings.

Once the crate is published, install the CLI with:

```bash
cargo install sealtask
```

Set a custom API URL with `SEALTASK_API_URL` if you are not targeting the default hosted endpoint:

```bash
SEALTASK_API_URL=https://your-sealtask.example cargo run -p sealtask -- me
```

Library callers construct the runtime with `RuntimeClient::new(api_url)?` for
same-origin storage or `RuntimeClient::with_storage_origins(api_url, origins)?`
for an explicit cross-origin storage allowlist.

The default profile stores credentials in `~/.sealtask`. Named profiles keep
credentials and their daemon/keychain identity isolated beneath
`~/.sealtask/profiles/<name>`:

```bash
sealtask --profile build-agent --json auth status
SEALTASK_PROFILE=build-agent sealtask --json tasks list --all
```

Use `--config-dir <path>` or `SEALTASK_CONFIG_DIR` to relocate the base state
directory for CI or sandboxed agents. `auth status` and `info` report the
resolved profile and directory.

## Development Notes

- The CLI defaults to table/text output for humans, including mutation results.
  Pass `--json` for compact machine output or `--format json-pretty` for
  indented JSON.
- Every public command and option has generated help. `schema` exposes the same
  command tree as versioned JSON for agents.
- Read commands return decrypted agent-facing models by default; raw wire DTOs are only available through hidden debug flags.
- Use `--non-interactive` whenever prompting would be unsafe. Non-interactive
  task creation requires a stable idempotency key. Use
  `auth unlock --password-stdin` for a temporary in-memory session, or
  `auth keychain store --password-stdin` to persist a local bootstrap secret in
  the platform keychain.
- Structured JSON inputs are exclusive with scalar editing flags and reject
  unknown fields instead of silently ignoring them.
- Password unlock supports legacy password-wrapped version 1 accounts and OPAQUE export-key version 2 accounts. Version 2 password unlock contacts the authenticated API; later daemon- or keychain-backed commands do not repeat that exchange.
- `tasks get` includes typed attachment metadata and lists attachment IDs in table output.
- `tasks attachments read` prints readable attachments to stdout, including plain text passthrough and DOCX rendered as Markdown; with `--json` it emits the rendered content plus attachment metadata.
- `tasks attachments download` decrypts binary attachments and saves them locally; if `--output` is omitted it writes `./<attachment-file-name>`. Output paths are current-working-directory-relative, reject absolute paths, parent traversal, and symbolic-link/reparse-point escapes, and do not overwrite an existing file unless `--force` is supplied.
- `tasks attachments upload` and `delete` update both the encrypted task payload and the API attachment ID set with optimistic concurrency protection.
- `notes` exposes decrypted list/get and encrypted create/update/delete for shared and private notes.
- The current workspace targets encrypted SealTask flows, so authenticated reads and writes still depend on credentials, local key unwrap, and workspace keys from a live SealTask deployment.
- CI for this repository runs from `.github/workflows/ci.yml`.
- Crates.io release steps are documented in [`RELEASE.md`](./RELEASE.md), with a helper script at [`scripts/publish-crates.sh`](./scripts/publish-crates.sh).

## Repository Flow

This public repository is mirrored automatically from SealTask's upstream
development repository with fast-forward-only updates. Public changes must be
ported back upstream before the next mirror; otherwise publication intentionally
stops instead of overwriting them. Release tags are write-once.

## Legal review

The StrongBox fork and the browser bindings are licensed under
`GPL-3.0-only`. This source publication is an engineering compliance step, not
legal advice. SealTask's counsel should confirm the obligations that apply to
hosted web delivery and licensed self-hosted distributions.

## License

This workspace is licensed under `GPL-3.0-only`. See [LICENSE](./LICENSE).
