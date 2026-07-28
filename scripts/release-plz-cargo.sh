#!/usr/bin/env bash
set -euo pipefail

# release-plz 0.3.160 uses CARGO for all subprocesses. In git-only mode it
# snapshots each prior release with this exact whole-workspace command:
#
#   cargo package --allow-dirty --workspace
#
# The workspace also contains two unpublished browser-WASM source packages.
# They are intentionally not part of the crates.io release graph, and Cargo
# cannot package their path-only fork dependency for registry publication.
#
# Keep every other Cargo invocation byte-for-byte equivalent. If a future
# release-plz changes the workspace-package command, fail closed so this
# adapter is reviewed alongside the upgrade.

if ! command -v cargo >/dev/null 2>&1; then
  echo "error: the pinned toolchain's Cargo executable was not found" >&2
  exit 2
fi

if [[ "$#" -eq 3 &&
      "$1" == "package" &&
      "$2" == "--allow-dirty" &&
      "$3" == "--workspace" ]]; then
  echo "release-plz Cargo adapter: excluding unpublished StrongBox WASM packages" >&2
  exec cargo \
    package \
    --allow-dirty \
    --workspace \
    --exclude strong-box \
    --exclude strong-box-wasm
fi

if [[ "${1:-}" == "package" ]]; then
  for argument in "$@"; do
    if [[ "${argument}" == "--workspace" ]]; then
      echo "error: release-plz used an unexpected whole-workspace package command" >&2
      printf 'received:' >&2
      printf ' %q' "$@" >&2
      printf '\n' >&2
      exit 2
    fi
  done
fi

exec cargo "$@"
