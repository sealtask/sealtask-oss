#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
MODE="${1:-update}"
ASSET_DIR="${ROOT_DIR}/cli/assets"

if [[ -z "${SEALTASK_BINARY:-}" ]]; then
  SEALTASK_BINARY="${ROOT_DIR}/target/debug/sealtask"
  cargo build \
    --manifest-path "${ROOT_DIR}/Cargo.toml" \
    --locked \
    -p sealtask
elif [[ ! -x "${SEALTASK_BINARY}" ]]; then
  printf 'SEALTASK_BINARY is not executable: %s\n' "${SEALTASK_BINARY}" >&2
  exit 1
fi

STAGING_DIR="$(mktemp -d "${TMPDIR:-/tmp}/sealtask-cli-assets.XXXXXXXX")"
trap 'rm -rf "${STAGING_DIR}"' EXIT
mkdir -p "${STAGING_DIR}/completions" "${STAGING_DIR}/man"

"${SEALTASK_BINARY}" completion bash > "${STAGING_DIR}/completions/sealtask.bash"
"${SEALTASK_BINARY}" completion zsh > "${STAGING_DIR}/completions/_sealtask"
"${SEALTASK_BINARY}" completion fish > "${STAGING_DIR}/completions/sealtask.fish"
"${SEALTASK_BINARY}" completion powershell > "${STAGING_DIR}/completions/sealtask.ps1"
"${SEALTASK_BINARY}" man --output-dir "${STAGING_DIR}/man" >/dev/null

case "${MODE}" in
  update)
    mkdir -p "${ASSET_DIR}/completions" "${ASSET_DIR}/man"
    find "${ASSET_DIR}/completions" -type f -delete
    find "${ASSET_DIR}/man" -type f -name '*.1' -delete
    cp "${STAGING_DIR}/completions/"* "${ASSET_DIR}/completions/"
    cp "${STAGING_DIR}/man/"*.1 "${ASSET_DIR}/man/"
    printf 'Updated SealTask CLI assets in %s\n' "${ASSET_DIR}"
    ;;
  check)
    if ! diff -ru "${ASSET_DIR}" "${STAGING_DIR}"; then
      printf 'SealTask CLI assets are stale; run ./scripts/generate-cli-assets.sh update\n' >&2
      exit 1
    fi
    printf 'SealTask CLI assets are current.\n'
    ;;
  *)
    printf 'usage: %s [update|check]\n' "$0" >&2
    exit 2
    ;;
esac
