# Changelog

All notable changes to the SealTask CLI and public Rust client crates are
documented here. The seven published crates share one version and one changelog.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project follows [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.5.0] - 2026-11-04

### Added

- add project-scoped agent control plane
- *(oss)* simplify CLI installation
- *(cli)* support task references across OSS workflows
- *(cli)* publish first-party agent skill
- *(cli)* scope project selection by directory
- *(cli)* add encrypted offline browsing
- *(cli)* add resilient batch workflows
- *(cli)* add live operator streams
- *(cli)* enrich task list workflows
- *(cli)* add secure editor workflows
- *(cli)* add private fuzzy picking
- *(cli)* add adaptive terminal output
- *(cli)* add shell-native discovery
- *(cli)* add operator diagnostics and request tracing
- *(cli)* add project workflows and human selectors
- *(cli)* make operator workflows safe and actionable
- harden public browser crypto release
- extract browser crypto engine
- publish browser WASM source from OSS workspace
- finalize OSS CLI v1 with machine-readable JSON and remove legacy backend CLI
- add encrypted notes and attachment workflows
- rebrand Worklist to SealTask across OSS workspace
- harden authenticated task automation
- support OPAQUE v2 CLI unlock
- add TOTP two-factor authentication
- *(oss)* replace --format with --json flag and add structured JSON output
- *(oss)* add client-runtime crate with full decrypted read flows
- *(oss)* prepare workspace for crates.io publication
- *(oss)* scaffold OSS client workspace with CLI and SDK crates

### Fixed

- *(cli)* match backend refresh rejection contract (worklist-x0ev)
- *(cli)* recover server-revoked sessions on login (worklist-x0ev)
- *(cli)* guard reused and partially saved logins (worklist-x0ev)
- *(cli)* allow login without a desktop keychain (worklist-x0ev)
- *(cli)* preserve sign-in across session rotation (worklist-x0ev)
- renew CLI sessions and unlock after login (worklist-x0ev)
- *(cli)* preserve parser format across argument terminator (worklist-xve)
- *(cli)* keep file input cancellable (worklist-t60)
- restore dependency audit and Linux CLI checks (worklist-ubl)
- keep schema discovery independent of config (worklist-pwj)
- disable paging in non-interactive mode (worklist-0wf)
- keep mutation stdin reads cancellable (worklist-i7q)
- harden CLI parser output (worklist-xmd)
- install the workspace WASM target
- allow supported Rust patch releases
- refresh generated CLI assets
- resolve audited dependency paths and guard exceptions (worklist-4pem)
- isolate agent process tree ownership
- *(cli)* make root help canonical
- harden OSS provenance publishing

### Other

- upgrade pinned Rust to 1.97
- *(cli)* name batch scheduler state
- *(agent)* start reconciliation deadline after setup (worklist-x0ev)
- *(agent)* encapsulate claim supervision context
- *(agent)* name prepared claim phase
- *(oss)* defer Windows release artifacts
- prepare OSS v0.2.1
- rebrand WorkList to SealTask across all files
- Revert "chore: rebrand WorkList to SealTask across all files"
- rebrand WorkList to SealTask across all files

## [0.4.0] - 2026-07-28

### Added

- *(oss)* simplify CLI installation

## [0.3.0] - 2026-07-28

### Added

- Added JSON contract v2, compact and pretty schema discovery, isolated named
  profiles, and deterministic non-interactive behavior for automation.
- Added operator-focused project selection, fuzzy picking, diagnostics, live
  streams, secure editor flows, resilient batches, and task-reference tools.
- Added reproducible Linux and macOS binary releases with a shell installer,
  checksums, SBOMs, and build attestations.

### Changed

- Made the six public Rust crates a lockstep graph with exact internal version
  requirements and resumable, checksum-verified crates.io publication.
## [0.2.1] - 2026-07-25

### Security

- Removed the unmaintained browser allocator override and tightened the
  canonical StrongBox WASM release checks.

## [0.2.0] - 2026-07-22

### Added

- Added the public MFA-compatible CLI and Rust client surface.

[Unreleased]: https://github.com/sealtask/sealtask-oss/compare/v0.5.0...HEAD
[0.5.0]: https://github.com/sealtask/sealtask-oss/compare/v0.4.0...v0.5.0
[0.4.0]: https://github.com/sealtask/sealtask-oss/compare/v0.3.0...v0.4.0
[0.3.0]: https://github.com/sealtask/sealtask-oss/compare/v0.2.1...v0.3.0
[0.2.1]: https://github.com/sealtask/sealtask-oss/compare/v0.2.0...v0.2.1
[0.2.0]: https://github.com/sealtask/sealtask-oss/releases/tag/v0.2.0
