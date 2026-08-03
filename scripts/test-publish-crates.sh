#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PUBLISH_SCRIPT="${SCRIPT_DIR}/publish-crates.sh"
BASH_BIN="${BASH:-bash}"
TEST_ROOT="$(mktemp -d "${TMPDIR:-/tmp}/sealtask-publish-tests.XXXXXX")"
PASSED=0

CRATES=(
  "sealtask-client-core"
  "sealtask-client-auth"
  "sealtask-client-crypto"
  "sealtask-client-api"
  "sealtask-client-runtime"
  "sealtask-agent"
  "sealtask"
)

cleanup() {
  case "${TEST_ROOT}" in
    "${TMPDIR:-/tmp}"/sealtask-publish-tests.*)
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
  exit 1
}

pass_test() {
  PASSED=$((PASSED + 1))
  printf 'ok %d - %s\n' "${PASSED}" "$1"
}

assert_status() {
  local expected="$1"
  if [[ "${RUN_STATUS}" != "${expected}" ]]; then
    sed -n '1,240p' "${OUTPUT_FILE}" >&2
    fail_test "expected exit ${expected}, got ${RUN_STATUS}"
  fi
}

assert_success() {
  assert_status 0
}

assert_failure() {
  if [[ "${RUN_STATUS}" == "0" ]]; then
    sed -n '1,240p' "${OUTPUT_FILE}" >&2
    fail_test "expected command to fail"
  fi
}

assert_output_contains() {
  local expected="$1"
  if ! grep -F -- "${expected}" "${OUTPUT_FILE}" >/dev/null; then
    sed -n '1,240p' "${OUTPUT_FILE}" >&2
    fail_test "output did not contain: ${expected}"
  fi
}

assert_log_count() {
  local expected="$1"
  local pattern="$2"
  local log_file="$3"
  local actual

  if [[ -f "${log_file}" ]]; then
    actual="$(grep -c -E -- "${pattern}" "${log_file}" || true)"
  else
    actual=0
  fi
  if [[ "${actual}" != "${expected}" ]]; then
    [[ ! -f "${log_file}" ]] || sed -n '1,240p' "${log_file}" >&2
    fail_test "expected ${expected} matches for ${pattern}, got ${actual}"
  fi
}

fixture_checksum() {
  local crate="$1"
  printf 'archive:%s:1.2.3\n' "${crate}" | sha256_stream
}

sha256_stream() {
  if command -v sha256sum >/dev/null 2>&1; then
    sha256sum | awk '{print $1}'
  else
    shasum -a 256 | awk '{print $1}'
  fi
}

new_fixture() {
  local name="$1"

  CASE_ROOT="${TEST_ROOT}/${name}"
  REPO_DIR="${CASE_ROOT}/repo"
  BIN_DIR="${CASE_ROOT}/bin"
  STATE_DIR="${CASE_ROOT}/state"
  OUTPUT_FILE="${CASE_ROOT}/output.log"

  mkdir -p "${REPO_DIR}/scripts" "${BIN_DIR}" "${STATE_DIR}" "${CASE_ROOT}/tmp"
  cp "${PUBLISH_SCRIPT}" "${REPO_DIR}/scripts/publish-crates.sh"
  printf '[workspace]\nmembers = []\n' >"${REPO_DIR}/Cargo.toml"

cat >"${REPO_DIR}/scripts/generate-cli-assets.sh" <<'SH'
#!/usr/bin/env bash
set -euo pipefail
[[ "${1:-}" == "check" ]]
if [[ "${DRY_RUN:-0}" == "1" ]]; then
  [[ "${CARGO_NET_OFFLINE:-}" == "true" ]]
fi
SH

  cat >"${BIN_DIR}/cargo" <<'SH'
#!/usr/bin/env bash
set -euo pipefail

state="${MOCK_STATE:?}"
command_name="${1:-}"
shift || true

find_arg_value() {
  local wanted="$1"
  shift
  while (( $# > 0 )); do
    if [[ "$1" == "${wanted}" ]]; then
      [[ $# -ge 2 ]] || exit 91
      printf '%s\n' "$2"
      return 0
    fi
    shift
  done
  return 1
}

reject_unsafe_args() {
  local arg
  for arg in "$@"; do
    case "${arg}" in
      --allow-dirty|--token|--token=*)
        printf 'unsafe cargo argument: %s\n' "${arg}" >&2
        exit 92
        ;;
    esac
  done
}

require_arg() {
  local wanted="$1"
  shift
  local arg
  for arg in "$@"; do
    [[ "${arg}" != "${wanted}" ]] || return 0
  done
  printf 'missing cargo argument: %s\n' "${wanted}" >&2
  exit 93
}

reject_unsafe_args "$@"

case "${command_name}" in
  pkgid)
    crate="$(find_arg_value -p "$@")"
    printf 'pkgid:%s\n' "${crate}" >>"${state}/cargo.log"
    printf 'path+file:///fixture#%s@1.2.3\n' "${crate}"
    ;;
  package)
    target_dir="$(find_arg_value --target-dir "$@")"
    require_arg --locked "$@"
    require_arg --no-verify "$@"
    if [[ "${DRY_RUN:-0}" == "1" ]]; then
      require_arg --offline "$@"
    fi
    all_args="$*"
    mkdir -p "${target_dir}/package"
    while (( $# > 0 )); do
      if [[ "$1" == "-p" ]]; then
        [[ $# -ge 2 ]] || exit 91
        crate="$2"
        printf 'archive:%s:1.2.3\n' "${crate}" \
          >"${target_dir}/package/${crate}-1.2.3.crate"
        if command -v sha256sum >/dev/null 2>&1; then
          sha256sum "${target_dir}/package/${crate}-1.2.3.crate" | awk '{print $1}' \
            >"${state}/checksum.${crate}"
        else
          shasum -a 256 "${target_dir}/package/${crate}-1.2.3.crate" | awk '{print $1}' \
            >"${state}/checksum.${crate}"
        fi
        printf 'package:%s:%s\n' "${crate}" "${all_args}" >>"${state}/cargo.log"
        shift 2
      else
        shift
      fi
    done
    ;;
  publish)
    crate="$(find_arg_value -p "$@")"
    require_arg --locked "$@"
    printf 'publish:%s:%s\n' "${crate}" "$*" >>"${state}/cargo.log"

    if [[ -f "${state}/publish-fail-before-upload.${crate}" ]]; then
      exit "$(cat "${state}/publish-fail-before-upload.${crate}")"
    fi

    cp "${state}/checksum.${crate}" "${state}/registry.${crate}"
    if [[ -f "${state}/publish-fail-after-upload.${crate}" ]]; then
      exit "$(cat "${state}/publish-fail-after-upload.${crate}")"
    fi
    ;;
  *)
    printf 'unexpected cargo command: %s\n' "${command_name}" >&2
    exit 94
    ;;
esac
SH

  cat >"${BIN_DIR}/curl" <<'SH'
#!/usr/bin/env bash
set -euo pipefail

state="${MOCK_STATE:?}"
output_file=""
user_agent=""
url=""

while (( $# > 0 )); do
  case "$1" in
    --connect-timeout|--max-time|--output|--user-agent|--write-out)
      option="$1"
      [[ $# -ge 2 ]] || exit 95
      value="$2"
      case "${option}" in
        --output) output_file="${value}" ;;
        --user-agent) user_agent="${value}" ;;
      esac
      shift 2
      ;;
    --silent|--show-error|--location)
      shift
      ;;
    *)
      url="$1"
      shift
      ;;
  esac
done

[[ -n "${output_file}" ]] || exit 96
[[ "${user_agent}" == "sealtask-release-publisher/1 (+https://github.com/sealtask/sealtask-oss)" ]] \
  || exit 97

path="${url#*/crates/}"
crate="${path%%/*}"
version="${path#*/}"
printf 'lookup:%s:%s:%s\n' "${crate}" "${version}" "${user_agent}" >>"${state}/curl.log"

mode="normal"
[[ ! -f "${state}/mode.${crate}" ]] || mode="$(cat "${state}/mode.${crate}")"
case "${mode}" in
  network-error)
    printf 'simulated transport error\n' >&2
    exit 7
    ;;
  500)
    printf '{"errors":[{"detail":"simulated"}]}' >"${output_file}"
    printf '500'
    exit 0
    ;;
  malformed)
    printf '{"version":' >"${output_file}"
    printf '200'
    exit 0
    ;;
esac

if [[ -f "${state}/registry.${crate}" ]]; then
  if [[ -f "${state}/delay.${crate}" ]]; then
    remaining="$(cat "${state}/delay.${crate}")"
    if (( remaining > 0 )); then
      printf '%d\n' "$((remaining - 1))" >"${state}/delay.${crate}"
      printf '{"errors":[{"detail":"not yet visible"}]}' >"${output_file}"
      printf '404'
      exit 0
    fi
  fi

  checksum="$(cat "${state}/registry.${crate}")"
  printf '{"version":{"crate":"%s","num":"%s","checksum":"%s"}}' \
    "${crate}" "${version}" "${checksum}" >"${output_file}"
  printf '200'
else
  printf '{"errors":[{"detail":"not found"}]}' >"${output_file}"
  printf '404'
fi
SH

  cat >"${BIN_DIR}/sleep" <<'SH'
#!/usr/bin/env bash
set -euo pipefail
printf 'sleep:%s\n' "${1:-}" >>"${MOCK_STATE:?}/sleep.log"
SH

  chmod +x \
    "${REPO_DIR}/scripts/publish-crates.sh" \
    "${REPO_DIR}/scripts/generate-cli-assets.sh" \
    "${BIN_DIR}/cargo" \
    "${BIN_DIR}/curl" \
    "${BIN_DIR}/sleep"

  git -C "${REPO_DIR}" init -q
  git -C "${REPO_DIR}" config user.name "Release Test"
  git -C "${REPO_DIR}" config user.email "release-test@example.invalid"
  git -C "${REPO_DIR}" add Cargo.toml scripts
  git -C "${REPO_DIR}" commit -qm "test fixture"
  git -C "${REPO_DIR}" tag -am "release v1.2.3" v1.2.3
}

seed_registry_match() {
  local crate="$1"
  fixture_checksum "${crate}" >"${STATE_DIR}/registry.${crate}"
}

run_fixture() {
  set +e
  (
    cd "${REPO_DIR}"
    env \
      PATH="${BIN_DIR}:${PATH}" \
      MOCK_STATE="${STATE_DIR}" \
      TMPDIR="${CASE_ROOT}/tmp" \
      REGISTRY_API_BASE="https://registry.invalid/api/v1" \
      RELEASE_TAG="v1.2.3" \
      GITHUB_REF_TYPE="tag" \
      GITHUB_REF_NAME="v1.2.3" \
      WAIT_SECONDS="0" \
      MAX_ATTEMPTS="3" \
      "$@" \
      "${BASH_BIN}" scripts/publish-crates.sh
  ) >"${OUTPUT_FILE}" 2>&1
  RUN_STATUS=$?
  set -e
}

assert_publish_order() {
  local expected
  local actual

  expected="$(printf '%s\n' "${CRATES[@]}")"
  actual="$(sed -n 's/^publish:\([^:]*\):.*$/\1/p' "${STATE_DIR}/cargo.log")"
  if [[ "${actual}" != "${expected}" ]]; then
    sed -n '1,240p' "${STATE_DIR}/cargo.log" >&2
    fail_test "crates were not published in dependency order"
  fi

  first_publish="$(grep -n '^publish:' "${STATE_DIR}/cargo.log" | head -1 | cut -d: -f1)"
  last_package="$(grep -n '^package:' "${STATE_DIR}/cargo.log" | tail -1 | cut -d: -f1)"
  if (( first_publish <= last_package )); then
    fail_test "publication began before all archives were staged"
  fi
}

test_fresh_publish() {
  new_fixture fresh
  run_fixture
  assert_success
  assert_log_count 7 '^package:' "${STATE_DIR}/cargo.log"
  assert_log_count 7 '^publish:' "${STATE_DIR}/cargo.log"
  assert_publish_order
  assert_output_contains "All crates are published with checksums matching the staged archives."
  pass_test "fresh publication stages everything, then publishes in dependency order"
}

test_partial_resume() {
  new_fixture partial-resume
  seed_registry_match "${CRATES[0]}"
  seed_registry_match "${CRATES[1]}"
  run_fixture
  assert_success
  assert_log_count 7 '^package:' "${STATE_DIR}/cargo.log"
  assert_log_count 5 '^publish:' "${STATE_DIR}/cargo.log"
  assert_output_contains "Skipping ${CRATES[0]} 1.2.3"
  assert_output_contains "Skipping ${CRATES[1]} 1.2.3"
  pass_test "partial publication resumes after checksum-verified crates"
}

test_full_resume() {
  new_fixture full-resume
  for crate in "${CRATES[@]}"; do
    seed_registry_match "${crate}"
  done
  run_fixture
  assert_success
  assert_log_count 7 '^package:' "${STATE_DIR}/cargo.log"
  assert_log_count 0 '^publish:' "${STATE_DIR}/cargo.log"
  assert_log_count 7 '^lookup:' "${STATE_DIR}/curl.log"
  pass_test "fully published releases become a checksum-verified no-op"
}

test_checksum_mismatch() {
  new_fixture checksum-mismatch
  printf '%064d\n' 0 >"${STATE_DIR}/registry.${CRATES[0]}"
  run_fixture
  assert_failure
  assert_output_contains "registry checksum mismatch for ${CRATES[0]} 1.2.3"
  assert_log_count 0 '^publish:' "${STATE_DIR}/cargo.log"
  pass_test "an existing version with a different archive checksum fails closed"
}

test_uncertain_registry_responses() {
  new_fixture uncertain-responses
  printf '500\n' >"${STATE_DIR}/mode.${CRATES[0]}"
  run_fixture
  assert_failure
  assert_output_contains "Registry returned HTTP 500"
  assert_log_count 0 '^publish:' "${STATE_DIR}/cargo.log"

  printf 'malformed\n' >"${STATE_DIR}/mode.${CRATES[0]}"
  : >"${STATE_DIR}/cargo.log"
  : >"${STATE_DIR}/curl.log"
  run_fixture
  assert_failure
  assert_output_contains "Registry returned malformed metadata"
  assert_log_count 0 '^publish:' "${STATE_DIR}/cargo.log"

  printf 'network-error\n' >"${STATE_DIR}/mode.${CRATES[0]}"
  : >"${STATE_DIR}/cargo.log"
  : >"${STATE_DIR}/curl.log"
  run_fixture
  assert_failure
  assert_output_contains "Registry request failed"
  assert_log_count 0 '^publish:' "${STATE_DIR}/cargo.log"
  pass_test "HTTP, malformed, and transport failures are not mistaken for absence"
}

test_delayed_registry_visibility() {
  new_fixture delayed
  printf '2\n' >"${STATE_DIR}/delay.${CRATES[0]}"
  run_fixture
  assert_success
  assert_log_count 2 '^sleep:' "${STATE_DIR}/sleep.log"
  assert_output_contains "Waiting for ${CRATES[0]} 1.2.3 to appear"
  pass_test "registry propagation is polled until the staged checksum is visible"
}

test_registry_timeout() {
  new_fixture timeout
  printf '99\n' >"${STATE_DIR}/delay.${CRATES[0]}"
  run_fixture
  assert_failure
  assert_output_contains "Timed out waiting for ${CRATES[0]} 1.2.3"
  assert_log_count 1 '^publish:' "${STATE_DIR}/cargo.log"
  assert_log_count 2 '^sleep:' "${STATE_DIR}/sleep.log"
  pass_test "registry propagation timeout stops before dependent crates"
}

test_nonzero_after_upload() {
  new_fixture nonzero-after-upload
  printf '42\n' >"${STATE_DIR}/publish-fail-after-upload.${CRATES[0]}"
  printf '1\n' >"${STATE_DIR}/delay.${CRATES[0]}"
  run_fixture
  assert_success
  assert_output_contains "cargo publish exited 42"
  assert_output_contains "Treating ${CRATES[0]} 1.2.3 as published"
  assert_log_count 7 '^publish:' "${STATE_DIR}/cargo.log"
  assert_log_count 1 '^sleep:' "${STATE_DIR}/sleep.log"
  pass_test "a nonzero cargo exit after upload is polled and reconciled by checksum"
}

test_dry_run_is_offline() {
  new_fixture dry-run
  run_fixture DRY_RUN=1
  assert_success
  assert_log_count 7 '^package:.*--offline' "${STATE_DIR}/cargo.log"
  assert_log_count 0 '^publish:' "${STATE_DIR}/cargo.log"
  assert_log_count 0 '^lookup:' "${STATE_DIR}/curl.log"
  assert_output_contains "all 7 crates were packaged offline"
  pass_test "dry-run packages every crate offline without registry or publish calls"
}

test_release_tag_guards() {
  new_fixture release-tag-guards

  git -C "${REPO_DIR}" tag -d v1.2.3 >/dev/null
  run_fixture
  assert_failure
  assert_output_contains "required release tag v1.2.3 does not exist"
  assert_log_count 0 '^package:' "${STATE_DIR}/cargo.log"
  assert_log_count 0 '^publish:' "${STATE_DIR}/cargo.log"

  git -C "${REPO_DIR}" tag v1.2.3
  run_fixture
  assert_failure
  assert_output_contains "required release tag v1.2.3 must be annotated"
  assert_log_count 0 '^package:' "${STATE_DIR}/cargo.log"
  assert_log_count 0 '^publish:' "${STATE_DIR}/cargo.log"

  git -C "${REPO_DIR}" tag -d v1.2.3 >/dev/null
  git -C "${REPO_DIR}" tag -am "release v1.2.3" v1.2.3
  git -C "${REPO_DIR}" commit --allow-empty -qm "advance past release tag"
  run_fixture
  assert_failure
  assert_output_contains "HEAD must be exactly tagged v1.2.3"
  assert_log_count 0 '^package:' "${STATE_DIR}/cargo.log"
  assert_log_count 0 '^publish:' "${STATE_DIR}/cargo.log"

  pass_test "missing, lightweight, and non-HEAD release tags fail before staging"
}

test_preflight_guards() {
  new_fixture preflight

  run_fixture DRY_RUN=true
  assert_failure
  assert_output_contains "DRY_RUN must be 0 or 1"

  run_fixture MAX_ATTEMPTS=0
  assert_failure
  assert_output_contains "MAX_ATTEMPTS must be a positive integer"

  run_fixture WAIT_SECONDS=-1
  assert_failure
  assert_output_contains "WAIT_SECONDS must be a non-negative integer"

  run_fixture API_MAX_TIME_SECONDS=1.5
  assert_failure
  assert_output_contains "API_MAX_TIME_SECONDS must be a positive integer"

  run_fixture RELEASE_TAG=
  assert_failure
  assert_output_contains "RELEASE_TAG must identify the immutable tag"

  run_fixture RELEASE_TAG=v9.9.9
  assert_failure
  assert_output_contains "does not match workspace version"

  printf '# dirty\n' >>"${REPO_DIR}/Cargo.toml"
  run_fixture DRY_RUN=1
  assert_failure
  assert_output_contains "working tree must be clean"

  pass_test "numeric, tag, and clean-tree preconditions fail before publication"
}

test_fresh_publish
test_partial_resume
test_full_resume
test_checksum_mismatch
test_uncertain_registry_responses
test_delayed_registry_visibility
test_registry_timeout
test_nonzero_after_upload
test_dry_run_is_offline
test_release_tag_guards
test_preflight_guards

printf '1..%d\n' "${PASSED}"
