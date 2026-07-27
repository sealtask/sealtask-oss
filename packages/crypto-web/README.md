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

Structured payload encryptors validate plaintext and return only sealed `bytes`
and `base64`. The package deliberately does not expose an unkeyed digest of the
plaintext.

Project membership keys have a product-qualified v2 plaintext envelope:

```text
{ kind: "sealtask-project-key", version: 2, key: <32 bytes> }
```

`sealWorkListKeyForOwner` is the compatibility-named writer. It defaults to
exactly 32 raw plaintext bytes so ciphertexts remain readable by previously
shipped clients that treated the decrypted plaintext as the key itself.
Callers may select `projectKeyWriteFormat: "v2-envelope"` only after a
fleet-wide minimum-client-version or equivalent rollout gate has excluded
those readers. `decryptWorkListKeyCiphertext` reads both write formats plus the
actually shipped CBOR byte-string, exact 32-element unsigned-byte array, and
single-field `{ key }` forms. The
abandoned `seal-work-list-key` draft was never committed or shipped and is
deliberately rejected; the v2 discriminator is `sealtask-project-key`.

## Trust protocol

Invite-key publications use owner-authorized transparency statement v2. The
package derives a deterministic Ed25519 identity from the account data key and
user UUID. It first requires the canonical lowercase hyphenated UUID, removes
the hyphens, and runs HKDF-SHA256 with the exact 32-byte data key as input
keying material, an empty salt, UTF-8 info
`transparency:owner-signing:v2:<32-lowercase-hex-uuid-digits>`, and a 32-byte
output. That output is the Ed25519 seed and is not clamped. The shared
Rust/browser compatibility corpus records the data key, UUID bytes, info,
seed, and public key as a reusable cross-client known-answer vector. Statement
hashing separately uses the 16 raw UUID bytes in the canonical sequence

```text
"worklist.transparency.statement.v2"
|| user_uuid[16]
|| generation_u64_be
|| invite_public_key_length_u32_be
|| invite_public_key[32]
|| identity_public_key[32]
|| predecessor_present_u8
|| previous_statement_digest[32]?
```

and signs the 32-byte statement digest. A verifier requires the complete
per-user chain from generation `0` through the target, contiguous generations,
valid v2 signatures and predecessor links, a stable owner identity, Merkle
inclusion for the target and first v2 identity statement, and consistency with
retained and release-pinned state.

The public API separates uses that must not be interchanged:

- `resolveCurrentInviteKey({ userId })` is the only result suitable for
  encrypting a new invitation. It requires the complete ordered directory
  snapshot for the same root, reconstructs that root and every required
  prefix, validates each per-user history, and selects the target user's
  maximum generation.
- `verifyHistoricalInviteKey({ userId, generation })` verifies the exact
  generation recorded in an existing invitation authenticator. It is not a
  current-recipient-key capability.

Legacy v1 statements are available only through
`inspectInviteKeyForMigration`, allowing their owner to sign a linked v2
successor. Current and historical ordinary verification, current-proof
ingestion, and invitation acceptance reject a legacy target. Likewise,
ordinary acceptance requires a modern invitation whose sender authenticator
verifies; decryptability alone is insufficient.

`TransparencyClient` requires both a `TransparencyIdentityStore` and one of:

- a nonzero `trustedCheckpoint` supplied by a reviewed release or independent
  operator; or
- `allowUnanchoredBootstrap: true`, explicitly, for development only.

The old automatic genesis-recovery options are rejected. Store implementations
must preserve valid checkpoints and owner-identity pins across logout and fail
closed when state is corrupt, cannot be persisted, conflicts with a trusted
root, or changes identity. Application code is responsible for durable storage;
the SealTask web client uses one account-independent bounded `localStorage`
checkpoint history for the global log and per-user owner-identity pins. Web
Locks and compare-and-swap writes prevent another tab from erasing newer trust
state.

Migration from the former per-account checkpoint keys preserves old entries as
evidence. It reconciles automatically only when every legacy account's latest
root still appears in the winning bounded 32-root history. Accounts whose
honest histories are farther apart fail closed and require evidence-preserving
manual recovery until an asynchronous consistency-proof migration is added.

Ordinary first use of a v2 identity requires its first v2 leaf to be covered by
the independently released checkpoint, unless the identity was already pinned
or the caller is verifying its own exact publication. A production rollout
must therefore migrate identities, audit the resulting history, and ship a new
reviewed anchor before enabling registered invitations. The checked-in pre-v2
anchor is continuity evidence, not an invitation-ready identity directory.

This security release intentionally breaks callers that publish v1 statements,
accept legacy invitations, construct a transparency client without an identity
store/trust policy, or consume structured encrypt results' former `schemaHash`
property. First-party request builders never transported `schemaHash`, so its
removal has no API or persistence migration.

The boundary is not absolute malicious-server resistance. Initial owner
identity publication is self-signed. A reviewed checkpoint can witness and
freeze that choice but does not retrospectively prove human account ownership.
The bounded full-directory proof exposes logged UUIDs, public keys, signatures,
and timestamps to authenticated current-key resolvers and is capped, together
with publication, at 20,000 statements. It must be replaced by a
privacy-preserving witnessed authenticated head map before that limit. A
release anchor is not a live witness for post-anchor forks. A malicious web
origin can replace delivered JavaScript. Data-key rotation needs an explicit
authorized identity-transition/recovery protocol and is not silently accepted.

React, TanStack Query, generated API clients, localization, monitoring, and
navigation remain application concerns. The frontend supplies those concerns
through narrow adapters and imports this package through compatibility shims;
the landing-page encryption demonstration imports the relevant package exports
directly.

## Development

From the public repository root:

```bash
bun install
bun run check:crypto-web
bun run --cwd packages/crypto-web test:browser
cargo run -p sealtask-client-crypto \
  --example generate_compat_fixtures -- --check
```

The StrongBox worker consumes the canonical WASM artifact produced and verified
by `scripts/build-strong-box-wasm.sh`.

The checked compatibility corpus at `../../testdata/crypto-compat-v1.json` is
generated by the Rust `sealtask-client-crypto` reference implementation and
consumed byte-for-byte by the browser tests.

New HPKE envelopes use RFC 9180's X25519 KEM ID `0x0020`. Decryption retains
fixture-backed support for SealTask's two historical `0x0010` dialects, but no
old writer exists.

The early plaintext-CBOR decoder, migration logic, and fixtures are private
application remediation code. They are intentionally absent from this package,
and the package boundary check rejects their introduction. Current `cbor-x`
encoding remains because it is part of the live encrypted protocol.

The current Git/workspace package resolves its StrongBox byte from the
repository-level canonical artifact. Before an npm release, the publish build
must copy that byte into the npm tarball and rewrite the worker asset import;
publishing the current source-only file list would omit it.
