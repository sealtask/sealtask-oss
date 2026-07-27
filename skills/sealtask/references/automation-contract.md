# SealTask automation contract

Use this reference to establish a safe machine session and choose commands.
Do not treat it as exhaustive CLI help; query the installed binary for current
syntax.

## Preflight

First choose the intended working directory. For a Git repository, resolve its
canonical root with `git rev-parse --show-toplevel` or an equivalent
repository-aware mechanism, then run every project-context command from that
root. If no repository root exists, use the workspace directory identified by
the user. Do not create local context from an incidental nested directory.

From that directory, run:

```bash
command -v sealtask
sealtask --json --non-interactive info
sealtask --json --non-interactive auth status
sealtask --json --non-interactive projects current
```

Require `jsonContractVersion` 2 from `info`. Keep stdout and stderr separate.
Successful finite commands emit one JSON document on stdout. Warnings and
runtime failures emit a structured envelope on stderr. Collections emit arrays,
including `[]`.

Do not run `doctor` on every request. Run
`sealtask --json --non-interactive doctor` when authentication, unlock,
configuration, API connectivity, or project context is unhealthy. Do not add
`--include-keychain`; it may trigger an operating-system prompt.

Authentication currently uses the operator's user principal. Prefer an
operator-established unlock daemon or keychain session. If the status says
login or unlock is required, stop and request that operator action. Do not
attempt to discover credentials or ask the user to paste secrets into chat.

## Output modes

- Use `--json --non-interactive` for bounded reads and mutations.
- Use `--format jsonl --non-interactive` for `tasks watch`,
  `activity follow`, and `batch run`.
- Treat help and version output as human text.
- Use `sealtask --json --non-interactive schema <COMMAND ...>` for the
  versioned visible command tree and argument metadata.
- Do not combine JSON output with `-v`, `-vv`, or `--debug`.
- Do not parse tables, colors, progress displays, pager output, prompts, or
  prose hints when structured fields are available.

For offline reads, add `--offline` explicitly. Offline mode never falls back to
the network and rejects mutations before sending a request.

## Project context

Resolve context in this order:

1. An explicit project selector on the command.
2. The nearest current-directory or ancestor local context.
3. The active profile's global fallback.
4. Cross-project assigned-task behavior where the command supports it.

Inspect the effective context from the intended workspace root with:

```bash
sealtask --json --non-interactive projects current
```

If no project is selected, list projects:

```bash
sealtask --json --non-interactive projects list
```

Choose only an unambiguous project that follows from the user's request. Ask
when multiple projects remain plausible. From that same root, activate an
explicit selector without opening a terminal picker:

```bash
sealtask --json --non-interactive pick project id:<project-id> --scope local
```

Prefer `--scope local` for repository work. Use `--scope global` only when the
user requested a profile-wide default. Project context selects a target; it
does not grant access.

## Selectors and inputs

Names and unique ID prefixes are useful for discovery. Once a resource is
resolved, retain and use the full `id:` selector from structured output.

Before composing an unfamiliar command, inspect it:

```bash
sealtask --json --non-interactive schema tasks create
```

Use structured input files for compound task, note, or comment bodies. Reject
unknown fields rather than guessing. `--input-stdin` and `--password-stdin`
cannot share stdin; rely on a pre-established unlock when structured data uses
stdin.

Treat decrypted stdout as sensitive. Avoid copying bodies, comments, private
notes, attachment contents, tokens, paths, or identifiers into unrelated logs
or chat responses.
