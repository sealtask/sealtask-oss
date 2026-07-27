# SealTask OSS

Open-source workspace for the `sealtask` CLI, shared Rust client crates, and
the browser cryptography engine. The canonical repository is
[`sealtask/sealtask-oss`](https://github.com/sealtask/sealtask-oss).

This repository contains the early public client surface for SealTask:

- `sealtask`: command-line client for authenticating and working with decrypted projects, tasks, comments, notes, and task attachments
- `sealtask-client-core`: shared public types and error handling
- `sealtask-client-auth`: local credential storage and authentication helpers
- `sealtask-client-api`: typed HTTP client for the SealTask API
- `sealtask-client-crypto`: client-side crypto helpers for sealed payloads and key derivation
- `sealtask-client-runtime`: unlock-aware runtime that projects raw API responses into agent-facing decrypted models
- `strong-box`: SealTask's GPL-3.0 StrongBox fork used by the browser engine
- `strong-box-wasm`: the Rust-to-WASM bindings shipped in the SealTask browser client
- `@sealtask/crypto-web`: the TypeScript browser runtime, encrypted payload
  protocols, and trust verification code used by the production SPA

## Status

This workspace is still in active development and is not yet positioned as a stable public SDK.

- crate boundaries are intentional, but APIs may still change
- several APIs may still evolve as the agent workflow surface expands
- the current release target is the CLI first, with supporting crates published alongside it

## 0.2.1 security note (2026-07-25)

Version 0.2.1 removes the unmaintained `wee_alloc` override from the browser
WASM bridge and uses Rust's supported default WASM allocator. It also updates
Hickory DNS, `rpassword`, QUIC protocol, WebPKI, and `anyhow` lockfile entries
past their published advisories.

HTTP control-plane requests now explicitly use Reqwest's operating-system
resolver. Attachment storage keeps its patched Hickory resolver for
pre-connection address validation and pins the validated addresses into the
dedicated no-proxy, no-redirect transfer client.

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
packages/crypto-web/     # production browser crypto and trust engine
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
- Bun for the browser crypto package

Common commands:

```bash
cargo check --workspace --all-targets --locked
cargo test --workspace --all-targets --locked
cargo run -p sealtask-client-crypto --example generate_compat_fixtures -- --check
./scripts/build-strong-box-wasm.sh build
./scripts/build-strong-box-wasm.sh verify
bun install --frozen-lockfile
bun run check:crypto-web
bun run --cwd packages/crypto-web test:browser
cargo audit --deny warnings --file Cargo.lock
cargo run -p sealtask -- --help
cargo run -p sealtask -- auth unlock --password-stdin
cargo run -p sealtask -- auth keychain store --password-stdin
cargo run -p sealtask -- projects list
cargo run -p sealtask -- projects use "Release Engineering"
cargo run -p sealtask -- tasks get "Prepare release notes"
cargo run -p sealtask -- --json tasks list --all
```

## Operator-friendly CLI workflows

`projects` is the canonical command name. The historical `lists` spelling
remains a visible compatibility alias, and existing API/JSON fields such as
`workListId` remain stable. Select a profile-local current project once to omit
the project scope from subsequent task, note, comment, and attachment commands:

```bash
sealtask projects list
sealtask projects use "Release Engineering"
sealtask projects current
sealtask tasks list
sealtask projects clear
```

Use `sealtask projects list --details` for expanded human-readable project
metadata. The former `--verbose` spelling remains accepted for compatibility,
but is hidden from help and generated discovery assets.

Task lists show their effective scope on an interactive terminal: an explicitly
selected project, the saved current project, or assigned tasks across projects.
Across-project tables include a `Project` column by default, and empty results
suggest commands appropriate to the active filters. Redirected and JSON output
omit that interactive context; JSON collection output remains the same stable
array contract.

Customize human tables with an exact, ordered column list and a natural sort:

```bash
sealtask tasks list --all --columns project,title,due,status --sort due
sealtask tasks list --columns id,title,priority,comments,updated --sort priority
```

Columns are comma-separated or repeatable and must be unique. Supported columns
are `id`, `title`, `project`, `project-id`, `priority`, `due`, `status`,
`comments`, `created`, and `updated`. Explicitly requested columns retain caller
order and stay present even on a narrow terminal. Text, project, due date, and
status sorts are ascending; priority sorts highest first; created and updated
sort newest first. `--columns` applies only to table output and is rejected with
JSON.

For shell composition, `--field id|title|url` writes exactly one sanitized value
per task, one per line, with no heading, total, scope, or empty-state text:

```bash
sealtask tasks list --field id
sealtask tasks list --field title
SEALTASK_WEB_URL=https://app.example sealtask tasks list --all --field url
sealtask tasks list --field url --web-url https://app.example
```

IDs are emitted as reusable full `id:<32-lowercase-hex>` selectors. URLs point
to `/workspace/work-lists/<project-id>?task=<task-id>`. The web origin must be
an absolute credential-free HTTP(S) origin without a path, query, or fragment;
when it is not configured, the CLI derives the origin from `SEALTASK_API_URL`.
`--web-url` is valid only with `--field url`; `SEALTASK_WEB_URL` is consulted
only for that field and does not affect other list modes. Raw-field output
rejects JSON and paging so pipes remain predictable.

### Shell completion, help, and manual pages

Generate native completion scripts without reading operator configuration,
credentials, decrypted project/task names, or the network:

```bash
# Bash or Zsh, for the current shell
source <(sealtask completion bash)
source <(sealtask completion zsh)

# Fish
sealtask completion fish | source

# PowerShell
sealtask completion powershell > sealtask.ps1
```

Package managers should install these generated files into their shell-specific
completion directories. `sealtask man` renders roff for the root command,
accepts canonical or aliased nested paths, and can generate the complete manual
set:

```bash
sealtask man tasks create
sealtask man lists get       # renders sealtask-projects-get(1)
sealtask man --output-dir ./target/man
```

Version-matched copies live in this repository under
`cli/assets/{completions,man}` and are included as `assets/...` in the published
crate archive. Maintainers regenerate and verify them with
`./scripts/generate-cli-assets.sh update` and
`./scripts/generate-cli-assets.sh check`.

Long help groups target, field, input, output, safety, and advanced options and
ends every runnable leaf command with copyable examples. `completion` and `man`
emit raw artifacts, so they reject JSON, terminal-presentation, quiet, and
diagnostic-verbosity flags that would corrupt or obscure stdout.

### Terminal output policy

Human output adapts to the terminal while machine output stays byte-stable:

- `--color auto|always|never` defaults to color only on the destination TTY.
  `NO_COLOR` (when non-empty), `CLICOLOR=0`, and `TERM=dumb` disable automatic
  color; an explicit `--color always` overrides them. JSON never contains ANSI.
- `--pager auto|always|never` pages only long human output by default.
  `--no-pager` is the explicit short form of `--pager never`. The pager command
  is the first configured value of `SEALTASK_PAGER`, then `PAGER`, then the
  platform default. An empty configured value disables paging.
- `--progress auto|always|never` controls delayed, phase-only indicators on
  stderr. Automatic progress requires human output plus terminal stdout and
  stderr, and never claims a byte percentage the runtime cannot measure.
- `-q` / `--quiet` suppresses automatic paging, progress, and successful mutation
  acknowledgements. Requested read data, JSON results, warnings, and errors
  remain visible.

The mode defaults can also be set with `SEALTASK_COLOR`,
`SEALTASK_PAGER_MODE`, and `SEALTASK_PROGRESS`. Redirected output is plain,
unpaged, and animation-free in automatic mode:

```bash
NO_COLOR=1 sealtask tasks list
SEALTASK_PAGER='less -R' sealtask tasks get "Release checklist"
sealtask --no-pager tasks list > tasks.txt
sealtask --quiet tasks complete "Publish artifacts"
```

Pager values are parsed into a program and argument vector and launched
directly—never through a shell. Decrypted output is passed only on pager stdin,
not in arguments or a temporary file. A user-configured pager therefore sees
the same decrypted content that would otherwise be printed to the terminal.

### Private fuzzy picking

`pick` provides fuzzy discovery without sending decrypted names through a pipe,
temporary file, shell completion, or external selector process:

```bash
project="$(sealtask pick project)"
sealtask projects get "$project"

task="$(sealtask pick task)"
sealtask tasks get "$task"

task="$(sealtask pick task --project "Operations")"
sealtask tasks get "$task" --project "Operations"
```

The search interface reads and writes the controlling terminal, even while
stdout is captured by command substitution. On success stdout contains exactly
one reusable `id:<32-lowercase-hex>` selector and a newline; decrypted titles
remain confined to the in-process picker and terminal display. `pick project`
shows active projects by default, while `--include-archived` expands the set.
`pick task` uses the saved current project unless `--project` is supplied, and
offers `--include-completed` and `--include-archived`.

Because that single-line selector is a raw composition protocol, `pick` rejects
JSON output, `--non-interactive`, and forced paging before fetching candidates.
It never discovers or invokes `fzf` (or another external chooser), and generated
shell completion remains static so decrypted names are never fetched while the
shell is completing a command. Pass an exact UUID, `id:<prefix>`, or exact name
directly in automation.

### Release D8: encrypted read cache and offline mode

Successful online project, task, comment, and note reads automatically refresh
a query-specific encrypted snapshot for the active profile. Repeated identical
reads within one invocation are memoized in memory. Ordinary online mode always
treats the API as authoritative: it never silently falls back to a persistent
snapshot when the network or server fails. A cache-write failure is reported as
a warning after the authoritative result has been returned.

Use `--offline` only when stale-but-explicit local data is preferable to a
network attempt:

```bash
# Populate the exact views while online.
sealtask projects list
sealtask tasks list --include-completed
sealtask notes list

# Later, read only matching encrypted snapshots.
sealtask --offline projects list
sealtask --offline tasks list --include-completed
sealtask --offline notes list
sealtask --offline browse
```

Snapshot keys include the command scope and relevant filters, so an offline
miss asks the operator to reconnect and run that exact read online first. Every
successful offline command that consumes cached data reports the capture time
and age of each distinct snapshot it used and confirms that no network request
was attempted. These warnings make staleness visible but are not rollback
protection.

The offline allowlist is deliberately narrow:

- cached reads: `projects` / `projects list`, `projects get`,
  `projects sections list`, `tasks list`, `tasks get`, `comments list`,
  `notes list`, and `notes get`
- attended cached discovery: `pick project`, `pick task`, and `browse`
- local cache and session inspection: `cache status`, `cache verify`,
  `cache clear`, `auth status`, `doctor`, and `projects current`
- local discovery and configuration: `info`, `schema`, `config`, `profile`,
  `completion`, and `man`

Everything else is rejected before authentication or network setup. This
includes task/comment/note/project mutations, project selection changes,
login/logout/unlock actions, `me`, `stats`, project audit and raw API modes,
attachments, activity and watch streams, and batch execution. Remove
`--offline` explicitly to run one of those commands.

Inspect and maintain the cache without exposing its contents:

```bash
sealtask cache status
sealtask --offline cache verify
printf '%s\n' "$SEALTASK_PASSWORD" \
  | sealtask --offline cache verify --password-stdin
sealtask cache clear
```

`cache status` reports presence, ciphertext size, and modification time without
decrypting or prompting. `cache verify` authenticates, decrypts, and validates
the complete bounded cache before reporting its schema, entry count, and
capture range. `cache clear` removes the active profile's encrypted cache and
also clears invocation-local copies.

`browse` is a read-only, in-process terminal view for choosing a project and
task and viewing its description, checklist, comments, and attachment names.
It requires an attended controlling TTY and rejects `--non-interactive`.
Decrypted labels and documents never enter redirected stdout or stderr, are
never handed to another process, and are not stored in a plaintext temporary
file. Unix renders directly through `/dev/tty`. Windows renders through an
attended stderr console handle and refuses to start if stdin or stderr is
redirected. Use `sealtask browse` for authoritative online reads and
`sealtask --offline browse` for cached reads.
On Unix, scoped SIGINT, SIGTERM, SIGHUP, and SIGQUIT handling clears the
CLI-owned terminal region, restores the cursor and prior handlers, and exits
with the standard interrupted status.

The snapshot content is one bounded StrongBox ciphertext with no plaintext
content or query-metadata sidecar. A private lock file contains only an opaque
invalidation generation used to prevent concurrent reads from resurrecting
snapshots after a mutation or account transition. The cache encryption key and
authenticated context are bound to the normalized API URL, account UUID,
active profile, and current decoded data-key ciphertext bytes, so moving a
cache across any of those boundaries fails authentication. At-rest protection
still depends on keeping the decrypted account data key unavailable: a process,
unlock daemon, or OS keychain entry that can supply that key can also decrypt
the cache. Offline mode never runs the OPAQUE version 2 export-key exchange, so
a version 2 account cannot use a password alone while offline; it needs an
already available unlock-daemon or OS-keychain data key. The format does not
prevent replacement with an older valid cache, and `cache clear` cannot
guarantee secure deletion from copy-on-write filesystems, snapshots, or
backups.

### Secure editor and body-file input

Long-form titles and Markdown bodies can stay out of shell history and process
arguments:

```bash
sealtask tasks create --edit
sealtask tasks create --title "Release checklist" --body-file ./checklist.md
sealtask tasks edit "Release checklist"
sealtask notes edit "Incident runbook"
sealtask comments create "Ship release" --body-file -
```

Editor documents use a deliberately small format: the first line is the title,
an optional single blank line separates it from the remaining Markdown body.
For task creation, `--title`, `--body`, or `--body-file` seed the editor while
priority, due date, section, and idempotency flags remain ordinary structured
options. Existing task and note edits are revision-checked, so a concurrent
change fails instead of being overwritten after a long editing session.

SealTask resolves `SEALTASK_EDITOR`, then `VISUAL`, then `EDITOR`, with `vi` on
Unix and Notepad on Windows as platform fallbacks. Editor command arguments are
parsed and executed directly—never through a shell—and only a generic temporary
path is appended. The child uses the controlling terminal, keeping JSON stdout
clean. Each document lives in a private temporary directory with a mode-0600
file on Unix; the whole directory, including adjacent swap or backup files, is
removed before any API mutation. On Unix, SIGINT, SIGTERM, SIGHUP, and SIGQUIT
are forwarded to the editor while the CLI retains control long enough to reap
it, remove the plaintext workspace, and return exit 130. An editor configured
to save recovery data elsewhere remains responsible for protecting that data.

`--body-file PATH` reads raw UTF-8 Markdown, while `--body-file -` reads stdin.
The stdin form cannot be combined with `--password-stdin`; use a saved unlock,
the unlock daemon, or a real body file in that case. Editor workflows reject
`--non-interactive`; body-file workflows are designed for automation.

Selectors accept an exact UUID, an exact or Unicode-normalized name, or a
unique UUID prefix of at least eight hexadecimal digits. Use `name:<value>` or
`id:<prefix>` to make the intended selector form explicit. Ambiguous selectors
fail with deterministic, plaintext-free ID candidates instead of choosing
silently:

```bash
sealtask tasks get "Prepare release notes"
sealtask tasks get id:019f42ab
sealtask notes get name:"Release checklist"
```

Comment and attachment mutation flags are ID-only selectors and accept the same
unique UUID prefixes shown in human tables:

```bash
sealtask comments update "Ship 0.4" --comment-id id:019f42ab --body "Approved"
sealtask tasks attachments read "Ship 0.4" --attachment-id id:019f42ab
```

At least eight hexadecimal characters are required. Comment prefix discovery
uses authenticated API metadata without decrypting comment bodies; attachment
prefix discovery reads the encrypted task payload. A full UUID remains an exact
fast path.

Discover section names before creating or moving a task:

```bash
sealtask projects sections list
sealtask tasks create --title "Ship 0.3" --section "In progress" \
  --priority p1 --due tomorrow
```

Priority aliases are `low`/`p4`/`1`, `medium`/`p3`/`3`,
`high`/`p2`/`5`, and `urgent`/`p1`/`8`. Human due dates such as `today`,
`tomorrow`, `YYYY-MM-DD`, and `YYYY-MM-DDTHH:MM` use the selected project's
timezone. Local times repeated during a daylight-saving transition are rejected;
use `--due-at` with an explicit RFC 3339 offset to choose the intended instant.
Archived tasks are discoverable with
`sealtask tasks list --include-completed --include-archived` in a selected
project.
Unlock durations accept compound values such as `30m`, `8h`, `1h30m`, and
`2d`.

### Diagnostics and operator configuration

Use `doctor` for a safe, actionable view of local state and API availability:

```bash
sealtask doctor
sealtask --json doctor
sealtask doctor --offline
sealtask doctor --strict
sealtask doctor --include-keychain
```

The default run checks operator settings, configuration, credentials, session
and unlock state, the unauthenticated API health endpoint, and authenticated
identity when a usable session exists. It still emits a remediation report when
`operator-settings.json` is corrupt or was written by an unsupported version.
`--offline` guarantees that neither API probe runs.
Failed checks exit unsuccessfully; `--strict` also makes warnings unsuccessful.
Keychain access is opt-in because `--include-keychain` may trigger an
operating-system prompt. JSON output is a versioned report with stable check
IDs, error codes, and remediation text.

Inspect effective configuration and manage the persisted default profile with:

```bash
sealtask config show
sealtask config show --resolved
sealtask --json config show --resolved
sealtask profile list
sealtask profile use build-agent
```

`config show --resolved` adds the source of each effective value. Resolution
order is the command-line flag, the corresponding environment variable, the
persisted selection or profile setting where supported, and finally the
built-in default. A `--profile` or `SEALTASK_PROFILE` override remains active
for the current invocation of `profile use`; the command reports that override
so operators know to unset it. `profile list` shows both the effective profile
and the persisted default that will take effect after an override is removed.

The default profile stores credentials in `~/.sealtask`. Named profiles isolate
credentials, project context, and unlock state beneath
`~/.sealtask/profiles/<name>`. Use `--config-dir <path>` or
`SEALTASK_CONFIG_DIR` to relocate the base state directory:

```bash
sealtask --profile build-agent auth status
SEALTASK_PROFILE=build-agent sealtask tasks list --all
SEALTASK_CONFIG_DIR=/run/sealtask-agent sealtask config show --resolved
```

Control-plane timeouts are configurable per invocation or environment:

```bash
sealtask --connect-timeout 5s --read-timeout 30s \
  --request-timeout 1m tasks list --all

SEALTASK_CONNECT_TIMEOUT=5s \
SEALTASK_READ_TIMEOUT=30s \
SEALTASK_REQUEST_TIMEOUT=1m \
  sealtask tasks list --all
```

Durations accept `ms`, `s`, `m`, and `h`, including compounds such as
`1m30s`, and must be between `1ms` and `24h`. Defaults are `10s` to connect,
`30s` to read, and `1m` for the whole request; connect and read timeouts cannot
exceed the request timeout. Best-effort old-session revocation during login and
logout honors shorter request timeouts but remains capped at `5s` so local
authentication cleanup cannot hang an operator session.

For troubleshooting, `-v` writes redacted start/finish events to stderr;
`-vv` and `--debug` also include the safe API origin, configuration sources,
and resolved timeouts. Profile names, configuration paths, URL credentials and
paths, and request payloads are omitted. Telemetry never occupies stdout. Each
invocation uses its telemetry `invocation_id` as the
`x-request-id` for all control-plane requests, which makes client and server
logs correlatable; requests also identify the version with the stable
`sealtask-client-api/<version>` user agent. Diagnostic telemetry cannot be
combined with `--json` or either JSON `--format`, preserving the one-document
JSON process contract.

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

## Browser compatibility corpus

`testdata/crypto-compat-v1.json` freezes the persisted Rust/browser boundary
for data-key wrappers, recovery and StrongBox wrappers, sealed payloads and
proofs, tasks/comments/notes/attachments, invite bindings and authentication,
and transparency proofs. `sealtask-client-crypto` is the deterministic
reference generator; browser tests consume the checked bytes through the
canonical WASM bridge.

HPKE migration vectors are intentionally separate. New writes use RFC 9180's
X25519 KEM ID `0x0020`; the browser retains decrypt-only support for the two
historical `0x0010` SealTask dialects. The old plaintext-CBOR development
artifact is not part of the public browser package.

## Agent task automation

The supported automation mode acts through an authenticated user session. Log
in once, then either unlock workspace data for a bounded session or save an
unlock key in the platform keychain:

```bash
printf '%s\n' "$SEALTASK_PASSWORD" \
  | cargo run -p sealtask -- --json --non-interactive auth login \
      --email agent-user@example.com --password-stdin

printf '%s\n' "$SEALTASK_PASSWORD" \
  | cargo run -p sealtask -- --json --non-interactive auth unlock \
      --ttl 8h --password-stdin
```

Create a task with the native lifecycle fields and a retry key:

```bash
cargo run -p sealtask -- --json --non-interactive tasks create \
  --project "Release Engineering" \
  --title 'Prepare release notes' \
  --priority high \
  --start-at 2026-08-09T08:00:00Z \
  --due '2026-08-10T09:30' \
  --section 'In progress' \
  --idempotency-key 'agent:run-42:release-notes'
```

Numeric priorities remain `1`, `3`, `5`, or `8`, and `--due-at` remains the
RFC 3339 form for automation that already computes an exact instant.
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

Preview a task create or update with `--dry-run`. The CLI still authenticates,
resolves selectors, validates and normalizes fields, and prepares the encrypted
request, but prints a versioned `taskMutationPlan` whose `willMutate` field is
`false` instead of sending the mutation:

```bash
sealtask --json tasks create --input-file ./task.json --dry-run
sealtask --json tasks update id:019f42ab --priority urgent --dry-run
```

For resumable automation, `batch run` accepts strict JSON Lines schema version
1. Each line has a unique safe `operationId`, an explicit `project` selector,
and either a `task.create` input or a `task.update` task selector and input:

```json
{"schemaVersion":1,"operationId":"release-42:create-notes","type":"task.create","project":"id:019f0000-0000-7000-8000-000000000001","input":{"title":"Prepare release notes","priority":5}}
{"schemaVersion":1,"operationId":"release-42:mark-urgent","type":"task.update","project":"id:019f0000-0000-7000-8000-000000000001","task":"id:019f0000-0000-7000-8000-000000000002","input":{"priority":8}}
```

Unknown fields and unsupported operation types are rejected. The entire input
is parsed and validated before authentication or mutation. One line is limited
to 4 MiB, the input to 64 MiB and 10,000 operations, and `--jobs` to 1–16.
Batch records cannot request passwords, editors, files, prompts, deletion, or
attachments. Checklist fields inside `input` retain the existing encrypted
payload spelling (`is_done`, `completed_at`, and `assignee_user_ids`).

```bash
sealtask batch run --input ./operations.jsonl --dry-run
sealtask --format jsonl batch run --input ./operations.jsonl --jobs 8 \
  --continue-on-error
sealtask --format jsonl batch run --input ./operations.jsonl \
  --checkpoint "$HOME/.local/state/sealtask/batch/release-42.json"
sealtask --format jsonl batch run --input ./operations.jsonl \
  --checkpoint "$HOME/.local/state/sealtask/batch/release-42.json" --resume
```

Table output prints sanitized per-operation lines and one summary for humans.
`--format jsonl` flushes versioned operation records and exactly one summary;
finite JSON formats are rejected. Exit status is `0` for complete success or a
successful dry run, `1` for validation/total failure, `3` for partial success
with `--continue-on-error`, `4` for checkpoint safety conflicts, and `130` for
interruption.

Checkpoints are bound to the canonical input SHA-256 and persist only hashed
operation IDs plus canonical UUID/revision/commitment metadata—never selectors,
task plaintext, ciphertext, encryption material, tokens, or idempotency values.
Durable checkpoint/resume is currently available on Linux and macOS, where
handle-relative atomic no-replace publication is available. Other platforms
fail closed before touching the checkpoint path, while ordinary batch
execution and dry runs remain available.
The parent directory chain created by the CLI is private (`0700` on POSIX);
checkpoint and lock files must be regular, non-symlink private files (`0600`).
Every supplied parent component must be a real directory rather than a
symlink, and `..` components are rejected before path normalization. On macOS,
use a real path such as `/private/tmp/...` instead of the `/tmp/...` alias when
keeping a checkpoint outside the home directory.
Transitions are appended to a bounded, fsynced JSONL journal under one
exclusive lock; recovery ignores only an incomplete final record, and
occasional atomic compaction keeps the journal at or below 8 MiB. New
checkpoints are published with one atomic no-replace rename, so neither an
existing checkpoint nor a crash-visible second hard link can be introduced.
A resume skips confirmed successes, safely replays creates through
project-keyed deterministic idempotency, and accepts a started update only
against its exact checkpointed revision and change commitment. Do not edit or
delete a checkpoint after an interrupted run until its in-flight state is
understood.

For ordinary one-shot project, task, comment, and note mutations, SIGINT or
SIGTERM cancels promptly while no durable request is in flight. If a mutation
request is active, the first signal waits up to 30 seconds for a definitive
response instead of dropping it. A second signal, or the grace deadline, exits
`130` with a typed `ambiguous` outcome only while that request is still active
and tells the operator to inspect the resource before retrying because the
mutation may have committed. If credentials are rotating when the first signal
arrives, the CLI first lets the replacement session persist, then cancels
before sending the requested resource mutation; forcing that boundary reports
the session outcome as ambiguous instead of claiming the resource changed.

For batches, the first signal stops new scheduling and safe preparation/retry
waits while allowing an already-started mutation and terminal checkpoint write
to reach a durable boundary. Credential rotation follows the same durable
session boundary: a forced second signal or grace timeout reports a
session-ambiguous outcome with login guidance and never claims that a resource
mutation was sent. A second signal otherwise forces bounded cleanup. Resume with
the exact same input and checkpoint after either interruption.

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
  --attachment-id <attachment-id> --yes
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
SIGINT and SIGTERM are observed before the upload and between its bounded
side-effecting stages. A storage PUT that has already started is awaited to its
bounded result before the CLI makes a bounded attempt to release the
server-side reservation. A second interrupt or cleanup-grace timeout returns
exit `130` with an `ambiguous` outcome so automation never treats an
unconfirmed reservation or link as a clean cancellation.

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

### Live tasks, activity, and audit

Follow one project's decrypted task state with an authoritative live view:

```bash
sealtask tasks watch --project "Release Engineering"
sealtask --format jsonl tasks watch --work-list-id <project-id>
```

The event stream is advisory. SealTask subscribes before loading the initial
snapshot, refetches authoritative tasks after every board or resync event, and
subscribes before refetching after a reconnect. Each reconnect uses a fresh
short-lived stream credential; credentials never appear in output, errors, or
debug values. Interactive terminals redraw only the CLI-owned region.
Redirected human output is append-only and contains no cursor control
sequences. JSONL refresh records carry the current authoritative `tasks`
snapshot plus filter-neutral `addedTaskIds`, `updatedTaskIds`, and
`removedTaskIds`; a completion or archive filtered from the view is therefore
never mislabeled as a deletion.

Account activity uses bounded cursor polling and emits initial history
oldest-first:

```bash
sealtask activity follow
sealtask activity follow --since 30m --interval 10s
sealtask --format jsonl activity follow
```

The default history window is 10 minutes and the default poll interval is five
seconds. Pagination, page size, history, and catch-up are bounded. Malformed
pagination or more than 1,000 unseen events fails explicitly instead of moving
the live anchor and silently dropping records.

Inspect one bounded project audit page without decrypting payload contents:

```bash
sealtask projects audit
sealtask projects audit "Release Engineering" --limit 25
sealtask --json projects audit --work-list-id <project-id>
```

Audit JSON uses an explicit safe projection. It reports `payloadPresent` but
never serializes the backend payload object, payload ciphertext, or payload
HMAC. Continuation output pins the project ID so a later current-project change
cannot apply a cursor to the wrong project.

Streaming commands reject `--json` and `--format json|json-pretty`, because a
never-ending stream cannot be one finite document. Use `--format jsonl` for one
compact, flushed, versioned domain record per stdout line. Stream warnings and
errors remain compact structured envelopes on stderr, paging is disabled, and
SIGINT or SIGTERM preserves the final display and exits with status 130.

### JSON process contract

`sealtask --json info` reports `"jsonContractVersion": 2`. For ordinary commands run
with `--json --non-interactive`, version 2 guarantees:

- `--json` emits one compact JSON document; `--format json-pretty` emits the
  same document with indentation; `--format jsonl` emits compact JSON Lines
  (one line for finite commands and a flushed record sequence for streams)
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
- permanent task, comment, note, and attachment deletion requires `--yes`;
  interactive table mode prompts when it can safely read from a terminal

Help and version output retain Clap's human-readable text format. Use
`sealtask --json schema [COMMAND ...]` for a versioned machine-readable command
and argument description.

Running `sealtask` without a command prints a short quick-start guide and exits
successfully; `sealtask --help` remains the complete command reference. In
interactive table mode, prompts are written to stderr. In JSON modes,
interactive prompts use the controlling terminal instead of stdout or stderr;
when no terminal is available, the command fails with a structured, actionable
error. Pass `--non-interactive` to make that policy explicit.

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
  `auth keychain store --password-stdin` to save an unlock key in the platform
  keychain.
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

This public repository is a read-only mirror of SealTask's upstream development
repository. Mirror updates are fast-forward-only and release tags are
write-once. Do not merge pull requests or push commits directly to public
`main`: even an identical patch would create different Git history and the
next safe mirror update would intentionally stop. See
[`CONTRIBUTING.md`](./CONTRIBUTING.md) for the proposal flow.

## Legal review

The StrongBox fork and the browser bindings are licensed under
`GPL-3.0-only`. This source publication is an engineering compliance step, not
legal advice. SealTask's counsel should confirm the obligations that apply to
hosted web delivery and licensed self-hosted distributions.

## License

This workspace is licensed under `GPL-3.0-only`. See [LICENSE](./LICENSE).
