---
name: sealtask
description: Manage SealTask projects, tasks, comments, notes, attachments, audits, and automation through the sealtask CLI. Use when the user explicitly mentions SealTask or sealtask, asks to inspect or change work tracked in SealTask, needs repository-local SealTask project context, wants CLI authentication or context diagnosis, or requests SealTask batch, watch, or offline workflows.
---

# SealTask

Operate through the installed `sealtask` binary. Treat the CLI as the source of
truth for commands and schemas. Act as the signed-in user; this skill does not
create an agent identity or grant additional authority. Treat directory project
context as a convenience, not an authorization boundary.

## Start every session

1. Determine the intended workspace root. For repository work, resolve the
   canonical repository root and run every context-sensitive command from
   there, even if the agent started in a nested directory.
2. Verify availability with `command -v sealtask` or the platform equivalent.
3. Run `sealtask --json --non-interactive info` and use its reported
   capabilities together with command schema discovery.
4. Run `sealtask --json --non-interactive auth status`.
5. From the intended workspace root, run
   `sealtask --json --non-interactive projects current` before a project-scoped
   read or mutation.

If authentication or workspace data is unavailable, ask the operator to sign
in or unlock it. Do not solicit, inspect, or relay account passwords,
authenticator codes, backup codes, tokens, decrypted keys, or keychain values.
Do not put secrets in command arguments or diagnostic output.

Read [references/automation-contract.md](references/automation-contract.md)
before the first operation in a session or whenever authentication, project
context, output mode, or command discovery is unclear.

## Choose commands dynamically

- Run finite commands with `--json --non-interactive`.
- Run streams and batches with `--format jsonl --non-interactive`.
- Query `sealtask --json --non-interactive schema <COMMAND ...>` before
  inventing flags, input fields, or enum values.
- Parse stdout and stderr separately. Branch on structured fields and process
  status, never on human message wording.
- Use task references for discovery when useful, then retain exact `id:`
  selectors returned by the CLI for mutations, checkpoints, and follow-up
  operations.

Never invoke a bare picker, browser, editor workflow, or any other attended
terminal UI. A full task reference such as `OPS-184` can infer its project when
no explicit or current project exists; a project-local selector such as `#184`
requires one. Quote `'#184'` in a shell, and use `name:OPS-184` for a literal
reference-shaped title. If no current project exists and the command cannot
infer one, list projects as JSON and resolve an unambiguous target. Ask the user
when the choice is ambiguous. From the workspace root, activate it with an
explicit ID and scope, normally:

```bash
sealtask --json --non-interactive pick project id:<project-id> --scope local
```

Never silently change global project context.

## Mutate safely

1. Confirm that the user's request authorizes the external mutation.
2. Read the target and retain its exact ID and current revision.
3. Use `--dry-run` when the command supports it and the intended change needs
   validation.
4. Create and persist one stable idempotency key per logical create before
   executing it. Reuse that key only for an identical retry.
5. Execute once and retain returned IDs.
6. Re-read after a conflict, reconcile the latest state, and retry only the
   reconciled change.
7. Inspect the resource before retrying an `ambiguous` outcome. Fetch a
   `committed` outcome instead of repeating the mutation.

Use `--yes` only when the user explicitly authorized that specific permanent
deletion. Never add blanket retries, automatic confirmations, or silent global
context changes.

Read [references/recovery.md](references/recovery.md) before handling a failed,
interrupted, destructive, or batched mutation.

## Report results

Summarize the action, project, affected resource IDs, and whether anything
changed. Surface actionable structured errors without exposing secrets or
unnecessarily reproducing decrypted task, note, comment, or attachment
contents.
