# Temporary cargo-audit applicability exceptions

Reviewed 2026-10-25; ZIP applicability rechecked 2026-11-02; SQLite lock entry
reviewed 2026-11-14; expires 2026-11-25. Follow-ups: `worklist-oule` (OPAQUE
dependencies), `worklist-3io4` (DOCX dependencies), and `worklist-rm1z` (sqlx
SQLite lock entry).

These advisories remain in the lockfiles. The exceptions concern operations
unused by this application; they do not claim that the dependencies are fixed.
All audit jobs retain `cargo audit --deny warnings` and first run
`scripts/check-cargo-audit-exceptions.py`. The guard checks the expiry, exact
ignore list, registry checksums, incoming dependency paths, OPAQUE cipher suite
declarations, the reviewed DOCX reader, and that no root manifest enables
SQLite. Its negative tests exercise drift and expiry. Any scope change requires a new applicability review, or removal of
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
[RUSTSEC-2026-0368](https://rustsec.org/advisories/RUSTSEC-2026-0368.html),
[RUSTSEC-2026-0369](https://rustsec.org/advisories/RUSTSEC-2026-0369.html), and
[RUSTSEC-2026-0429](https://rustsec.org/advisories/RUSTSEC-2026-0429.html).

The only application call uses `undocx::builder().skip_images().convert_bytes`
after bounded archive validation. The reviewed upstream conversion reads XML
entries in memory and does not call archive extraction. Disabling image export
avoids the converter's filesystem output path. Our direct ZIP dependency is
upgraded to 9.1.0; validation rejects absolute paths, traversal, symlinks,
excessive entry counts, oversized entries, and oversized aggregate content.
Regression coverage checks malicious paths and symlinks are rejected before
conversion and leave no output in a temporary canary directory.

The November 2 review covers RUSTSEC-2026-0429 specifically: the affected
`ZipArchive::extract` allocates a symlink-target buffer from an untrusted declared
size. `undocx`'s `converter/mod.rs` passes a byte cursor to `DocxFile::from_reader`;
`rs-docx`'s `docx.rs` reads named XML entries with `by_name` and `read_to_string`.
Neither conversion path calls `ZipArchive::extract`. Our validator also rejects
symlinks and bounds declared and actual entry sizes before conversion. The
existing checksum, call-site, source-hash, and expiry guards apply unchanged to
this additional extraction-only advisory. There is no patched release of
`undocx` or `rs-docx` available at the review date; their replacement remains
tracked in `worklist-3io4`.

The separate zstd findings are fixed by upgrading the OSS lockfile to `zstd
0.13.4`, `zstd-safe 7.4.0`, and `zstd-sys 2.1.2`. They have no audit exception.

The guard pins the upstream reader checksums, permits their legacy ZIP versions
only through these dependency routes, and pins the application reader file's
SHA-256. Changing the reader or introducing another `undocx` call site fails the
guard. This is a drift check supporting the source review, not a formal proof
of reachability. The root and iOS workspaces have no ZIP exception.

## libsqlite3-sys 0.37.0 (root workspace)

[RUSTSEC-2026-0459](https://rustsec.org/advisories/RUSTSEC-2026-0459.html)
concerns the SQLite C library bundled by `libsqlite3-sys` 0.6.0 to 0.38.2:
opening a database together with a crafted `-wal` file can return
uninitialized page-cache memory. The patched release is 0.39.0, but the latest
`sqlx-sqlite` (0.9.0) requires `libsqlite3-sys >=0.30.1, <0.38.0`.

The root lockfile records `libsqlite3-sys 0.37.0` only because Cargo's lockfile
resolution includes optional dependencies named by weak features
(`sqlx-sqlite?/…`). `sqlx 0.9.0` reaches `sqlx-sqlite 0.9.0` directly and
through `sqlx-macros 0.9.0` → `sqlx-macros-core 0.9.0`; `sqlx-sqlite` is the
only consumer of `libsqlite3-sys 0.37.0`.
The workspace enables only sqlx's Postgres driver. Cargo's feature resolver
never selects `sqlx-sqlite`: `cargo tree -i libsqlite3-sys --target all -e all
--all-features --workspace` prints nothing, and no build produces a SQLite
artifact. The application never opens SQLite databases.

The guard pins these five registry checksums and dependency routes, and
requires that only local workspace crates depend on `sqlx`. It rejects any
workspace or member manifest that enables a sqlx feature that turns on SQLite
(`sqlite*`, `all-databases`, `_unstable*`), directly, through a member's
`workspace = true` entry under a renamed `[workspace.dependencies]` key, or
through a `[features]` entry forwarding to such a key. It rejects any direct,
renamed, or target-specific dependency on `sqlx-sqlite`, `libsqlite3-sys`, or
`rusqlite`, and it fails closed when the lockfile shows a local `sqlx` consumer
whose manifest it did not inspect, such as a path dependency that Cargo adds to
the workspace without a `workspace.members` entry.
Any other affected `libsqlite3-sys` version fails in every scope. The OSS and
iOS workspaces have no SQLite exception.
