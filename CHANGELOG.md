# Changelog

All notable changes to the SealTask CLI and public Rust client crates are
documented here. The six published crates share one version and one changelog.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project follows [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

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

[Unreleased]: https://github.com/sealtask/sealtask-oss/compare/v0.3.0...HEAD
[0.3.0]: https://github.com/sealtask/sealtask-oss/compare/v0.2.1...v0.3.0
[0.2.1]: https://github.com/sealtask/sealtask-oss/compare/v0.2.0...v0.2.1
[0.2.0]: https://github.com/sealtask/sealtask-oss/releases/tag/v0.2.0
