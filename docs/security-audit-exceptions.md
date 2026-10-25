# Temporary cargo-audit applicability exceptions

Reviewed 2026-10-25; expires 2026-11-25. Follow-ups: `worklist-oule`
(OPAQUE dependencies) and `worklist-3io4` (DOCX dependencies).

These advisories remain in the lockfiles. The exceptions concern operations
unused by this application; they do not claim that the dependencies are fixed.
All audit jobs retain `cargo audit --deny warnings` and first run
`scripts/check-cargo-audit-exceptions.py`. The guard checks the expiry, exact
ignore list, registry checksums, incoming dependency paths, OPAQUE cipher suite
declarations, and the reviewed DOCX reader. Its negative tests exercise drift
and expiry. Any scope change requires a new applicability review, or removal of
the exception after an upstream fix.

## crypto-bigint 0.5.5

[RUSTSEC-2026-0387](https://rustsec.org/advisories/RUSTSEC-2026-0387.html)
concerns variable-time secret arithmetic. The sole remaining route is
`opaque-ke 4.0.1` / `voprf 0.5.0` → `elliptic-curve 0.13.8` →
`crypto-bigint 0.5.5`. Cargo also records OPAQUE's optional `ecdsa 0.16.9`
dependency. Every application OPAQUE suite (server, CLI, and iOS) selects
Ristretto255 for both OPRF and TripleDH. Its group arithmetic uses
`curve25519-dalek`; the shared elliptic-curve utilities do not select the
affected big-integer arithmetic. The guard pins these suite declarations and
rejects additional consumers of the old elliptic-curve, ECDSA, or bigint crates.

The active Web Push P-256 implementation now uses `p256 0.14` and patched
`crypto-bigint 0.7`. RFC 8291 payload encryption uses Mozilla's `ece` OpenSSL
backend. A regression test decrypts the message and independently verifies the
VAPID signature using OpenSSL. The R2 client no longer enables unused AWS
SigV4a; an offline presigning test checks its existing HMAC SigV4 behavior.

## Transitive ZIP readers

`undocx 0.5.2` uses `zip 2.6.1` directly and `zip 4.6.1` through
`rs-docx 0.2.0`. These releases are affected by filesystem extraction findings
[RUSTSEC-2026-0367](https://rustsec.org/advisories/RUSTSEC-2026-0367.html),
[RUSTSEC-2026-0368](https://rustsec.org/advisories/RUSTSEC-2026-0368.html), and
[RUSTSEC-2026-0369](https://rustsec.org/advisories/RUSTSEC-2026-0369.html).

The only application call uses `undocx::builder().skip_images().convert_bytes`
after bounded archive validation. The reviewed upstream conversion reads XML
entries in memory and does not call archive extraction. Disabling image export
avoids the converter's filesystem output path. Our direct ZIP dependency is
upgraded to 9.1.0; validation rejects absolute paths, traversal, symlinks,
excessive entry counts, oversized entries, and oversized aggregate content.
Regression coverage checks malicious paths and symlinks are rejected before
conversion and leave no output in a temporary canary directory.

The guard pins the upstream reader checksums, permits their legacy ZIP versions
only through these dependency routes, and pins the application reader file's
SHA-256. Changing the reader or introducing another `undocx` call site fails the
guard. This is a drift check supporting the source review, not a formal proof
of reachability. The root and iOS workspaces have no ZIP exception.
