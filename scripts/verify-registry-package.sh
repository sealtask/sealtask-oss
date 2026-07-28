#!/usr/bin/env bash
set -euo pipefail

fail() {
  printf 'error: %s\n' "$*" >&2
  exit 1
}

if (( $# != 2 )); then
  fail "usage: $0 BINARY EXPECTED_VERSION"
fi

BINARY="$1"
EXPECTED_VERSION="$2"

[[ -x "${BINARY}" ]] || fail "registry binary is not executable: ${BINARY}"
[[ -n "${EXPECTED_VERSION}" ]] || fail "expected version must not be empty"

OUTPUT_ROOT="$(mktemp -d "${TMPDIR:-/tmp}/sealtask-registry-verifier.XXXXXX")"
cleanup() {
  case "${OUTPUT_ROOT}" in
    "${TMPDIR:-/tmp}"/sealtask-registry-verifier.*)
      rm -rf -- "${OUTPUT_ROOT}"
      ;;
    *)
      printf 'Refusing to clean unexpected verifier path: %s\n' \
        "${OUTPUT_ROOT}" >&2
      ;;
  esac
}
trap cleanup EXIT

if ! "${BINARY}" --version >"${OUTPUT_ROOT}/actual-version"; then
  fail "registry binary failed to report its version"
fi
printf 'sealtask %s\n' "${EXPECTED_VERSION}" >"${OUTPUT_ROOT}/expected-version"
if ! cmp -s \
  "${OUTPUT_ROOT}/expected-version" \
  "${OUTPUT_ROOT}/actual-version"; then
  fail "registry binary version output did not exactly match sealtask ${EXPECTED_VERSION}"
fi

if ! "${BINARY}" \
  --json \
  --non-interactive \
  info \
  >"${OUTPUT_ROOT}/info.json"; then
  fail "registry binary failed to report its JSON contract"
fi
if ! jq -e -s '
  length == 1 and
  (.[0] | (
    type == "object" and
    .commandName == "sealtask" and
    .jsonContractVersion == 2
  ))
' "${OUTPUT_ROOT}/info.json" >/dev/null; then
  fail "registry binary returned an invalid JSON contract"
fi

printf 'Verified sealtask %s and JSON contract version 2.\n' "${EXPECTED_VERSION}"
