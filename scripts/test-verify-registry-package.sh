#!/usr/bin/env bash
set -euo pipefail

if [[ -n "${FAKE_SEALTASK_MODE:-}" ]]; then
  mode="${FAKE_SEALTASK_MODE}"

  if [[ "$*" == "--version" ]]; then
    case "${mode}" in
      version-mismatch) printf 'sealtask 9.9.9\n' ;;
      version-multiline) printf 'sealtask 0.3.0\nunexpected\n' ;;
      version-trailing-blank) printf 'sealtask 0.3.0\n\n' ;;
      version-failure) exit 42 ;;
      *) printf 'sealtask 0.3.0\n' ;;
    esac
    exit 0
  fi

  if [[ "$*" != "--json --non-interactive info" ]]; then
    printf 'unexpected arguments: %s\n' "$*" >&2
    exit 43
  fi

  case "${mode}" in
    valid|version-mismatch|version-multiline|version-trailing-blank|version-failure)
      printf '{"commandName":"sealtask","jsonContractVersion":2}\n'
      ;;
    wrong-command)
      printf '{"commandName":"another-cli","jsonContractVersion":2}\n'
      ;;
    missing-command)
      printf '{"jsonContractVersion":2}\n'
      ;;
    wrong-contract)
      printf '{"commandName":"sealtask","jsonContractVersion":1}\n'
      ;;
    missing-contract)
      printf '{"commandName":"sealtask"}\n'
      ;;
    malformed-json)
      printf '{"commandName":\n'
      ;;
    extra-json)
      printf '%s\n' \
        '{"commandName":"another-cli","jsonContractVersion":1}' \
        '{"commandName":"sealtask","jsonContractVersion":2}'
      ;;
    info-failure)
      exit 44
      ;;
    *)
      printf 'unknown fake mode: %s\n' "${mode}" >&2
      exit 45
      ;;
  esac
  exit 0
fi

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
VERIFIER="${SCRIPT_DIR}/verify-registry-package.sh"
TEST_ROOT="$(mktemp -d "${TMPDIR:-/tmp}/sealtask-registry-verifier-tests.XXXXXX")"
FAKE_BINARY="${BASH_SOURCE[0]}"
OUTPUT_FILE="${TEST_ROOT}/output.log"
PASSED=0

cleanup() {
  case "${TEST_ROOT}" in
    "${TMPDIR:-/tmp}"/sealtask-registry-verifier-tests.*)
      rm -rf -- "${TEST_ROOT}"
      ;;
    *)
      printf 'Refusing to clean unexpected test path: %s\n' "${TEST_ROOT}" >&2
      ;;
  esac
}
trap cleanup EXIT

fail_test() {
  printf 'not ok - %s\n' "$*" >&2
  sed -n '1,160p' "${OUTPUT_FILE}" >&2
  exit 1
}

pass_test() {
  PASSED=$((PASSED + 1))
  printf 'ok %d - %s\n' "${PASSED}" "$1"
}

run_case() {
  local mode="$1"
  local expected_status="$2"
  local description="$3"
  local status

  set +e
  FAKE_SEALTASK_MODE="${mode}" \
    "${VERIFIER}" "${FAKE_BINARY}" 0.3.0 >"${OUTPUT_FILE}" 2>&1
  status=$?
  set -e

  if [[ "${expected_status}" == "success" && "${status}" != "0" ]]; then
    fail_test "${description}: expected success, got exit ${status}"
  fi
  if [[ "${expected_status}" == "failure" && "${status}" == "0" ]]; then
    fail_test "${description}: expected failure"
  fi
  pass_test "${description}"
}

run_case valid success "exact version and JSON contract pass"
run_case version-mismatch failure "a different version fails"
run_case version-multiline failure "extra version output fails"
run_case version-trailing-blank failure "trailing blank version output fails"
run_case version-failure failure "a failing version command fails"
run_case wrong-command failure "a different command name fails"
run_case missing-command failure "a missing command name fails"
run_case wrong-contract failure "a different JSON contract version fails"
run_case missing-contract failure "a missing JSON contract version fails"
run_case malformed-json failure "malformed JSON fails"
run_case extra-json failure "multiple JSON documents fail"
run_case info-failure failure "a failing info command fails"

printf '1..%d\n' "${PASSED}"
