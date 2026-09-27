# SealTask transparency witness

Standalone continuity auditor for the SealTask public transparency monitor.
It verifies append-only history from an explicitly reviewed checkpoint, then
publishes a timestamped Ed25519 checkpoint using C2SP tlog-cosignature v1.0.1.
It does not receive customer keys, encrypted invitation packages, or database
credentials. It does not independently establish human account ownership.

From the OSS workspace, run `bun install --frozen-lockfile`, then
`bun run check:witness` and `bun run --cwd packages/transparency-witness build`.
The executable is `dist/transparency-witness` inside this package.

Supply a JSON configuration with `name`, `logOrigin`, `monitorUrl`,
`seed: { size, hash }`, `stateDirectory`, `publicDirectory`, and `privateKeyFile`.
The signing key is an Ed25519 PKCS8 PEM file created on an independent host.
Both directories must already exist. Never expose the private directory or key.

```sh
flock --nonblock /private/state/lock ./dist/transparency-witness config.json initialize
flock --nonblock /private/state/lock ./dist/transparency-witness config.json audit
```

Use `initialize` exactly once; it never overwrites existing state. Schedule
`audit` every five seconds. Every process accessing this state must hold the
same lock. Each audit verifies and atomically persists the private checkpoint
before publishing to the separate public directory. Missing/corrupt state,
rollback, conflicting proofs and unavailable monitors fail closed. Success
prints public size and timestamp; failure prints a fixed event and failure stage,
without raw error details, and exits 1.

Serve only `publicDirectory/checkpoint` over HTTPS with `Cache-Control: no-store`
and `Access-Control-Allow-Origin: *`. Alert on signature freshness, not just
HTTP availability. Keep a single active signer; do not reset state, restore an
older checkpoint or rotate keys automatically. Retain independent evidence to
reconcile disaster recovery. Client builds must pin the witness public key and
log origin; a runtime key obtained from the monitored log is not trusted.

This is a Worklist monitor adapter, not an implementation of the C2SP witness
submission API. It uses Worklist's existing Merkle consistency proof encoding,
including its prefix subtree hash. One independent witness has no quorum or
universal split-view guarantee. A malicious web origin can replace client code.
