#!/usr/bin/env bash
set -euo pipefail

fail() {
  printf 'error: %s\n' "$*" >&2
  exit 1
}

if (( $# != 2 )); then
  fail "usage: $0 DOWNLOADED_ARTIFACTS_DIR RELEASE_ASSETS_DIR"
fi

DOWNLOADED_ARTIFACTS_DIR="$1"
RELEASE_ASSETS_DIR="$2"

[[ -d "${DOWNLOADED_ARTIFACTS_DIR}" ]] \
  || fail "downloaded artifacts directory does not exist: ${DOWNLOADED_ARTIFACTS_DIR}"
[[ ! -e "${RELEASE_ASSETS_DIR}" ]] \
  || fail "release assets path already exists: ${RELEASE_ASSETS_DIR}"

EXPECTED_DOWNLOADS=(
  aarch64-apple-darwin-dist-manifest.json
  dist-manifest.json
  global-dist-manifest.json
  sealtask-aarch64-apple-darwin.tar.xz
  sealtask-aarch64-apple-darwin.tar.xz.sha256
  sealtask-client-api.cdx.xml
  sealtask-client-auth.cdx.xml
  sealtask-client-core.cdx.xml
  sealtask-client-crypto.cdx.xml
  sealtask-client-runtime.cdx.xml
  sealtask-installer.sh
  sealtask-x86_64-apple-darwin.tar.xz
  sealtask-x86_64-apple-darwin.tar.xz.sha256
  sealtask-x86_64-unknown-linux-gnu.tar.xz
  sealtask-x86_64-unknown-linux-gnu.tar.xz.sha256
  sealtask.cdx.xml
  sha256.sum
  source.tar.gz
  source.tar.gz.sha256
  strong-box-wasm.cdx.xml
  strong-box.cdx.xml
  x86_64-apple-darwin-dist-manifest.json
  x86_64-unknown-linux-gnu-dist-manifest.json
)
diff -u \
  <(printf '%s\n' "${EXPECTED_DOWNLOADS[@]}" | sort) \
  <(
    find "${DOWNLOADED_ARTIFACTS_DIR}" \
      -mindepth 1 \
      -maxdepth 1 \
      -type f \
      -exec basename {} \; |
      sort
  )

RELEASE_ASSETS=(
  dist-manifest.json
  sealtask-aarch64-apple-darwin.tar.xz
  sealtask-aarch64-apple-darwin.tar.xz.sha256
  sealtask-client-api.cdx.xml
  sealtask-client-auth.cdx.xml
  sealtask-client-core.cdx.xml
  sealtask-client-crypto.cdx.xml
  sealtask-client-runtime.cdx.xml
  sealtask-installer.sh
  sealtask-x86_64-apple-darwin.tar.xz
  sealtask-x86_64-apple-darwin.tar.xz.sha256
  sealtask-x86_64-unknown-linux-gnu.tar.xz
  sealtask-x86_64-unknown-linux-gnu.tar.xz.sha256
  sealtask.cdx.xml
  sha256.sum
  source.tar.gz
  source.tar.gz.sha256
  strong-box-wasm.cdx.xml
  strong-box.cdx.xml
)
mkdir "${RELEASE_ASSETS_DIR}"
for asset in "${RELEASE_ASSETS[@]}"; do
  cp \
    "${DOWNLOADED_ARTIFACTS_DIR}/${asset}" \
    "${RELEASE_ASSETS_DIR}/${asset}"
done
diff -u \
  <(printf '%s\n' "${RELEASE_ASSETS[@]}" | sort) \
  <(
    find "${RELEASE_ASSETS_DIR}" \
      -mindepth 1 \
      -maxdepth 1 \
      -type f \
      -exec basename {} \; |
      sort
  )

(
  cd "${RELEASE_ASSETS_DIR}"
  sha256sum --check sha256.sum
  for checksum in ./*.sha256; do
    sha256sum --check "${checksum}"
  done
)

jq -e '
  .announcement_tag == "v0.3.0" and
  .announcement_is_prerelease == false and
  (.announcement_title | length) > 0 and
  (.announcement_github_body | length) > 0 and
  (.releases | length) == 1 and
  .releases[0].app_name == "sealtask" and
  .releases[0].app_version == "0.3.0"
' "${RELEASE_ASSETS_DIR}/dist-manifest.json" >/dev/null

printf 'Prepared and verified 19 pinned v0.3.0 release assets.\n'
