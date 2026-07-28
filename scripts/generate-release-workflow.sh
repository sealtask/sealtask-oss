#!/usr/bin/env bash
set -euo pipefail

MODE="${1:-check}"
if [[ "${MODE}" != "check" && "${MODE}" != "update" ]]; then
  echo "usage: $(basename "$0") [check|update]" >&2
  exit 2
fi

SOURCE_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
DIST_BIN="${DIST_BIN:-dist}"
TEMP_DIR="$(mktemp -d)"
trap 'rm -rf "${TEMP_DIR}"' EXIT

if ! command -v "${DIST_BIN}" >/dev/null 2>&1; then
  echo "cargo-dist 0.32.0 is required to verify the release workflow" >&2
  exit 1
fi
if [[ "$("${DIST_BIN}" --version)" != "cargo-dist 0.32.0" ]]; then
  echo "expected cargo-dist 0.32.0" >&2
  exit 1
fi

tar \
  --exclude='./target' \
  --exclude='./.git' \
  -C "${SOURCE_DIR}" \
  -cf - . |
  tar -C "${TEMP_DIR}" -xf -

git -C "${TEMP_DIR}" init --quiet
git -C "${TEMP_DIR}" add .
git -C "${TEMP_DIR}" \
  -c user.name=dist-generator \
  -c user.email=dist-generator@example.invalid \
  commit --quiet -m baseline

# The checked-in workflow intentionally contains audited hardening that
# cargo-dist 0.32.0 cannot express, so runtime commands use allow-dirty=["ci"].
# Temporarily remove that escape hatch here and delete the copied workflow so
# this check still proves the raw generator output from first principles.
python3 - "${TEMP_DIR}/dist-workspace.toml" <<'PY'
from pathlib import Path
import sys

config = Path(sys.argv[1])
source = config.read_text(encoding="utf-8")
needle = 'allow-dirty = ["ci"]\n'
count = source.count(needle)
if count != 1:
    raise SystemExit(f"expected one cargo-dist CI allow-dirty entry, found {count}")
config.write_text(source.replace(needle, "", 1), encoding="utf-8")
PY
rm -f "${TEMP_DIR}/.github/workflows/release.yml"

(
  cd "${TEMP_DIR}"
  "${DIST_BIN}" generate --mode ci
)
python3 \
  "${SOURCE_DIR}/scripts/patch-dist-workflow.py" \
  "${TEMP_DIR}/.github/workflows/release.yml"

expected="${TEMP_DIR}/.github/workflows/release.yml"
actual="${SOURCE_DIR}/.github/workflows/release.yml"
if [[ "${MODE}" == "update" ]]; then
  cp "${expected}" "${actual}"
  echo "Updated ${actual}."
  exit 0
fi

if ! cmp -s "${expected}" "${actual}"; then
  echo "generated release workflow is stale; run:" >&2
  echo "  DIST_BIN=${DIST_BIN} ./oss/scripts/generate-release-workflow.sh update" >&2
  diff -u "${actual}" "${expected}" || true
  exit 1
fi

# Restore the runtime configuration and prove dist accepts the intentionally
# patched workflow exactly as the hosted plan job will.
cp "${SOURCE_DIR}/dist-workspace.toml" "${TEMP_DIR}/dist-workspace.toml"
(
  cd "${TEMP_DIR}"
  "${DIST_BIN}" plan --output-format=json >/dev/null
  workspace_version="$(
    python3 -c \
      'import pathlib, tomllib; print(tomllib.loads(pathlib.Path("Cargo.toml").read_text())["workspace"]["package"]["version"])'
  )"
  git \
    -c user.name=dist-generator \
    -c user.email=dist-generator@example.invalid \
    tag -a "v${workspace_version}" -m "Synthetic dist release rehearsal"
  "${DIST_BIN}" host \
    --steps=create \
    --tag="v${workspace_version}" \
    --output-format=json \
    >/dev/null
)
echo "Generated release workflow is current."
