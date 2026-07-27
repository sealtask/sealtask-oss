# SealTask mutation and recovery rules

Read this reference before destructive work, retries, interruption recovery, or
batch execution.

## One-shot mutations

1. Confirm that the user authorized the exact external change.
2. Read the target and retain its full ID and revision.
3. Resolve names and human dates before mutation; use `--dry-run` where
   supported when the result needs validation.
4. Persist a stable idempotency key before a logical create. Reuse it only for
   an identical retry.
5. Execute once and retain the returned ID.
6. Verify the resulting state when the operation is important or recovery was
   involved.

Permanent task, comment, note, and attachment deletion requires `--yes`. Supply
it only for a deletion the user explicitly requested.

## Structured failures

Read `error.code`, `error.retryable`, `error.retryAfterSeconds`,
`error.outcome`, `error.backendCode`, and `error.httpStatus`. Treat `message`
and `hint` as display text rather than branching keys.

| Condition | Required response |
| --- | --- |
| `validation` or process exit 2 | Query the command schema, correct the rejected input, and do not retry unchanged. |
| `authentication` | Ask the operator to sign in, then re-run preflight. |
| `mfa_input_required` | Ask the operator to complete login through a safe attended or secret-provisioned channel. |
| `conflict` | Re-read the resource, reconcile the current revision, and retry only the reconciled change. |
| `rate_limited` | Honor `retryAfterSeconds` when present. |
| `retryable: true` | Retry only when the operation is replay-safe or protected by the same persisted idempotency key. |
| `outcome: ambiguous` | Inspect the remote resource before retrying; the mutation may have committed. |
| `outcome: committed` | Fetch the committed resource; do not repeat the mutation. |
| `outcome: cancelled` | Confirm no mutation was in flight before deciding whether a new attempt is safe. |

Exit `130` means interruption, not ordinary failure. Inspect the structured
outcome before deciding whether anything can be repeated.

## Batch execution

Use strict JSON Lines input and give every operation a unique stable
`operationId`. Use explicit project and resource selectors in every record.
Dry-run the complete input first when practical.

For resumable work:

```bash
sealtask --format jsonl --non-interactive batch run \
  --input <operations.jsonl> \
  --checkpoint <checkpoint-path>
```

After interruption or partial failure, preserve the exact original input and
checkpoint. Resume with both unchanged:

```bash
sealtask --format jsonl --non-interactive batch run \
  --input <operations.jsonl> \
  --checkpoint <checkpoint-path> \
  --resume
```

Do not edit, replace, or delete the checkpoint until in-flight state is
understood. Exit `3` indicates partial failure, `4` a checkpoint safety
conflict, and `130` interruption. Inspect every emitted operation record and
the final summary before reporting success.
