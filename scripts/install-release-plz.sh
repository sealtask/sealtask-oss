#!/usr/bin/env bash
set -euo pipefail

if [[ "$#" != 1 ]]; then
  echo "usage: $0 BIN_DIR" >&2
  exit 2
fi

BIN_DIR="$1"
ARCHIVE="$(mktemp)"
trap 'rm -f "${ARCHIVE}"' EXIT
# release-plz v0.3.160, release-plz-x86_64-unknown-linux-gnu.tar.gz.
# Use the asset API rather than the release download URL that returns 504s in CI.
curl --proto '=https' --tlsv1.2 --fail --location --silent --show-error \
  --retry 3 --retry-delay 2 --retry-max-time 120 \
  --connect-timeout 10 --max-time 60 \
  --header 'Accept: application/octet-stream' \
  --output "${ARCHIVE}" \
  https://api.github.com/repos/release-plz/release-plz/releases/assets/476455741
echo "2263c4f95eac1513da96a114a77fde20ea038742a8c8050f7514b8f93b828646  ${ARCHIVE}" |
  sha256sum --check
mkdir -p "${BIN_DIR}"
tar -xzf "${ARCHIVE}" -C "${BIN_DIR}"
