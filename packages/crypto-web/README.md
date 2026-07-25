# `@sealtask/crypto-web`

`@sealtask/crypto-web` is the browser cryptography and trust engine shipped by
SealTask. The public Git source is canonical; the package is not yet published
to npm.

The package is framework-independent and organized into:

- `runtime`: byte handling, randomness, KDF/MAC primitives, OPAQUE, X25519,
  HPKE, and the StrongBox worker/WASM bridge
- `protocols`: encrypted payloads, keys, tasks, comments, notes, attachments,
  commitments, audit payloads, invite issuance, and validation
- `trust`: transparency proofs and checkpoint transitions plus authenticated
  invite verification and acceptance

React, TanStack Query, generated API clients, localization, monitoring, and
navigation remain application concerns. The frontend supplies those concerns
through narrow adapters and imports this package through compatibility shims.

## Development

From the public repository root:

```bash
bun install
bun run check:crypto-web
```

The StrongBox worker consumes the canonical WASM artifact produced and verified
by `scripts/build-strong-box-wasm.sh`.

The early plaintext-CBOR compatibility decoder remains private migration code
in the SealTask frontend and is intentionally not exported here.

The current Git/workspace package resolves its StrongBox byte from the
repository-level canonical artifact. Before an npm release, the publish build
must copy that byte into the npm tarball and rewrite the worker asset import;
publishing the current source-only file list would omit it.
