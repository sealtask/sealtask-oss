#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "${ROOT_DIR}"

DRY_RUN="${DRY_RUN:-0}"
WAIT_SECONDS="${WAIT_SECONDS:-10}"
MAX_ATTEMPTS="${MAX_ATTEMPTS:-30}"
API_CONNECT_TIMEOUT_SECONDS="${API_CONNECT_TIMEOUT_SECONDS:-10}"
API_MAX_TIME_SECONDS="${API_MAX_TIME_SECONDS:-30}"
REGISTRY_API_BASE="${REGISTRY_API_BASE:-https://crates.io/api/v1}"
REGISTRY_USER_AGENT="${REGISTRY_USER_AGENT:-sealtask-release-publisher/1 (+https://github.com/sealtask/sealtask-oss)}"

CRATES=(
  "sealtask-client-core"
  "sealtask-client-auth"
  "sealtask-client-crypto"
  "sealtask-client-api"
  "sealtask-client-runtime"
  "sealtask-agent"
  "sealtask"
)

fail() {
  printf 'error: %s\n' "$*" >&2
  exit 1
}

validate_zero_or_one() {
  local name="$1"
  local value="$2"

  case "${value}" in
    0|1) ;;
    *) fail "${name} must be 0 or 1 (received: ${value})" ;;
  esac
}

validate_nonnegative_integer() {
  local name="$1"
  local value="$2"

  if [[ ! "${value}" =~ ^(0|[1-9][0-9]*)$ ]]; then
    fail "${name} must be a non-negative integer (received: ${value})"
  fi
}

validate_positive_integer() {
  local name="$1"
  local value="$2"

  if [[ ! "${value}" =~ ^[1-9][0-9]*$ ]]; then
    fail "${name} must be a positive integer (received: ${value})"
  fi
}

validate_zero_or_one "DRY_RUN" "${DRY_RUN}"
validate_nonnegative_integer "WAIT_SECONDS" "${WAIT_SECONDS}"
validate_positive_integer "MAX_ATTEMPTS" "${MAX_ATTEMPTS}"
validate_positive_integer "API_CONNECT_TIMEOUT_SECONDS" "${API_CONNECT_TIMEOUT_SECONDS}"
validate_positive_integer "API_MAX_TIME_SECONDS" "${API_MAX_TIME_SECONDS}"

for executable in cargo curl git python3 sleep; do
  if ! command -v "${executable}" >/dev/null 2>&1; then
    fail "required executable was not found: ${executable}"
  fi
done

ensure_clean_tree() {
  local status

  status="$(git status --porcelain --untracked-files=all -- .)"
  if [[ -n "${status}" ]]; then
    printf '%s\n' "${status}" >&2
    fail "the OSS working tree must be clean before packaging or publishing"
  fi
}

crate_version() {
  local crate="$1"
  local pkgid
  local version

  pkgid="$(cargo pkgid --manifest-path Cargo.toml -p "${crate}")"
  version="${pkgid##*@}"
  [[ -n "${version}" && "${version}" != "${pkgid}" ]] \
    || fail "could not determine the version of ${crate} from cargo pkgid"
  printf '%s\n' "${version}"
}

sha256_file() {
  local archive="$1"

  if command -v sha256sum >/dev/null 2>&1; then
    sha256sum "${archive}" | awk '{print $1}'
  elif command -v shasum >/dev/null 2>&1; then
    shasum -a 256 "${archive}" | awk '{print $1}'
  elif command -v openssl >/dev/null 2>&1; then
    openssl dgst -sha256 -r "${archive}" | awk '{print $1}'
  else
    fail "sha256sum, shasum, or openssl is required to hash crate archives"
  fi
}

ensure_exact_release_tag() {
  local version="$1"
  local expected_tag="v${version}"
  local head_commit
  local tag_commit
  local tag_object_type

  if [[ -z "${RELEASE_TAG:-}" ]]; then
    fail "RELEASE_TAG must identify the immutable tag being published"
  fi
  if [[ "${RELEASE_TAG}" != "${expected_tag}" ]]; then
    fail "RELEASE_TAG=${RELEASE_TAG} does not match workspace version ${version}"
  fi

  if ! tag_object_type="$(
    git cat-file -t "refs/tags/${expected_tag}" 2>/dev/null
  )"; then
    fail "required release tag ${expected_tag} does not exist"
  fi
  if [[ "${tag_object_type}" != "tag" ]]; then
    fail "required release tag ${expected_tag} must be annotated"
  fi

  head_commit="$(git rev-parse --verify HEAD)"
  if ! tag_commit="$(git rev-parse --verify "refs/tags/${expected_tag}^{commit}" 2>/dev/null)"; then
    fail "annotated release tag ${expected_tag} does not resolve to a commit"
  fi
  if [[ "${tag_commit}" != "${head_commit}" ]]; then
    fail "HEAD must be exactly tagged ${expected_tag} before publishing"
  fi
}

# lookup_registry_checksum sets LOOKUP_CHECKSUM and returns:
#   0: version exists and has a valid checksum
#   4: the registry explicitly returned HTTP 404
#   5: registry state is uncertain (transport error, other HTTP status, bad JSON)
LOOKUP_CHECKSUM=""
lookup_registry_checksum() {
  local crate="$1"
  local version="$2"
  local response_file
  local error_file
  local http_code
  local curl_exit
  local checksum

  LOOKUP_CHECKSUM=""
  response_file="$(mktemp "${STAGING_ROOT}/registry-response.XXXXXX")"
  error_file="$(mktemp "${STAGING_ROOT}/registry-error.XXXXXX")"

  set +e
  http_code="$(
    curl \
      --silent \
      --show-error \
      --location \
      --connect-timeout "${API_CONNECT_TIMEOUT_SECONDS}" \
      --max-time "${API_MAX_TIME_SECONDS}" \
      --user-agent "${REGISTRY_USER_AGENT}" \
      --output "${response_file}" \
      --write-out '%{http_code}' \
      "${REGISTRY_API_BASE%/}/crates/${crate}/${version}" \
      2>"${error_file}"
  )"
  curl_exit=$?
  set -e

  if (( curl_exit != 0 )); then
    printf 'Registry request failed for %s %s (curl exit %d).\n' \
      "${crate}" "${version}" "${curl_exit}" >&2
    if [[ -s "${error_file}" ]]; then
      sed -n '1,3p' "${error_file}" >&2
    fi
    return 5
  fi

  case "${http_code}" in
    404)
      return 4
      ;;
    200)
      set +e
      checksum="$(
        python3 - "${response_file}" "${crate}" "${version}" <<'PY'
import json
import re
import sys

try:
    with open(sys.argv[1], "rb") as response:
        payload = json.load(response)
    published = payload["version"]
    checksum = published["checksum"]
except (OSError, json.JSONDecodeError, KeyError, TypeError) as error:
    print(f"invalid crates.io response: {error}", file=sys.stderr)
    raise SystemExit(1)

if published.get("crate") != sys.argv[2] or published.get("num") != sys.argv[3]:
    print("invalid crates.io response: crate name or version did not match", file=sys.stderr)
    raise SystemExit(1)

if not isinstance(checksum, str) or re.fullmatch(r"[0-9a-fA-F]{64}", checksum) is None:
    print("invalid crates.io response: version.checksum is not a SHA-256 digest", file=sys.stderr)
    raise SystemExit(1)

print(checksum.lower())
PY
      )"
      curl_exit=$?
      set -e
      if (( curl_exit != 0 )); then
        printf 'Registry returned malformed metadata for %s %s.\n' \
          "${crate}" "${version}" >&2
        return 5
      fi
      LOOKUP_CHECKSUM="${checksum}"
      return 0
      ;;
    *)
      printf 'Registry returned HTTP %s for %s %s; publication state is uncertain.\n' \
        "${http_code:-<empty>}" "${crate}" "${version}" >&2
      return 5
      ;;
  esac
}

require_matching_registry_checksum() {
  local crate="$1"
  local version="$2"
  local expected_checksum="$3"
  local lookup_status

  if lookup_registry_checksum "${crate}" "${version}"; then
    if [[ "${LOOKUP_CHECKSUM}" != "${expected_checksum}" ]]; then
      fail "registry checksum mismatch for ${crate} ${version}: expected ${expected_checksum}, found ${LOOKUP_CHECKSUM}"
    fi
    return 0
  else
    lookup_status=$?
  fi

  return "${lookup_status}"
}

wait_for_matching_registry_checksum() {
  local crate="$1"
  local version="$2"
  local expected_checksum="$3"
  local attempt=1
  local lookup_status

  while (( attempt <= MAX_ATTEMPTS )); do
    if require_matching_registry_checksum "${crate}" "${version}" "${expected_checksum}"; then
      printf 'Confirmed %s %s on crates.io with checksum %s.\n' \
        "${crate}" "${version}" "${expected_checksum}"
      return 0
    else
      lookup_status=$?
    fi

    if (( lookup_status != 4 )); then
      return "${lookup_status}"
    fi
    if (( attempt == MAX_ATTEMPTS )); then
      break
    fi

    printf 'Waiting for %s %s to appear on crates.io (%d/%d)...\n' \
      "${crate}" "${version}" "${attempt}" "${MAX_ATTEMPTS}"
    sleep "${WAIT_SECONDS}"
    ((attempt += 1))
  done

  printf 'Timed out waiting for %s %s to appear on crates.io with checksum %s.\n' \
    "${crate}" "${version}" "${expected_checksum}" >&2
  return 1
}

ensure_clean_tree
if [[ "${DRY_RUN}" == "1" ]]; then
  CARGO_NET_OFFLINE=true ./scripts/generate-cli-assets.sh check
else
  ./scripts/generate-cli-assets.sh check
fi
ensure_clean_tree

VERSIONS=()
for crate in "${CRATES[@]}"; do
  VERSIONS+=("$(crate_version "${crate}")")
done

RELEASE_VERSION="${VERSIONS[0]}"
for index in "${!CRATES[@]}"; do
  if [[ "${VERSIONS[${index}]}" != "${RELEASE_VERSION}" ]]; then
    fail "${CRATES[${index}]} has version ${VERSIONS[${index}]}; all published crates must share ${RELEASE_VERSION}"
  fi
done

if [[ "${DRY_RUN}" == "0" ]]; then
  ensure_exact_release_tag "${RELEASE_VERSION}"
fi

STAGING_ROOT="$(mktemp -d "${TMPDIR:-/tmp}/sealtask-crates.XXXXXX")"
cleanup_staging_root() {
  if [[ -n "${STAGING_ROOT:-}" && -d "${STAGING_ROOT}" ]]; then
    rm -rf -- "${STAGING_ROOT}"
  fi
}
trap cleanup_staging_root EXIT

STAGING_TARGET="${STAGING_ROOT}/target"
ARCHIVES=()
CHECKSUMS=()

package_args=(
  package
  --manifest-path Cargo.toml
  --locked
  --no-verify
  --target-dir "${STAGING_TARGET}"
)
if [[ "${DRY_RUN}" == "1" ]]; then
  package_args+=(--offline)
fi
for crate in "${CRATES[@]}"; do
  package_args+=(-p "${crate}")
done

# Packaging all workspace crates in one Cargo invocation is required for the
# first release: Cargo can then resolve unpublished path dependencies from the
# other selected packages instead of requiring them to exist on crates.io.
printf 'Staging all crate archives before release-state checks or publication.\n'
cargo "${package_args[@]}"

for index in "${!CRATES[@]}"; do
  crate="${CRATES[${index}]}"
  version="${VERSIONS[${index}]}"
  archive="${STAGING_TARGET}/package/${crate}-${version}.crate"
  [[ -f "${archive}" ]] \
    || fail "cargo did not create the expected archive: ${archive}"
  checksum="$(sha256_file "${archive}")"
  [[ "${checksum}" =~ ^[0-9a-fA-F]{64}$ ]] \
    || fail "could not calculate a valid SHA-256 digest for ${archive}"

  ARCHIVES+=("${archive}")
  checksum="$(printf '%s' "${checksum}" | tr '[:upper:]' '[:lower:]')"
  CHECKSUMS+=("${checksum}")
  printf 'Staged %s (sha256: %s).\n' "${archive}" "${checksum}"
done

ensure_clean_tree

if [[ "${DRY_RUN}" == "1" ]]; then
  printf '\nDry run completed successfully: all %d crates were packaged offline; no registry requests or publications were made.\n' \
    "${#CRATES[@]}"
  exit 0
fi

for index in "${!CRATES[@]}"; do
  crate="${CRATES[${index}]}"
  version="${VERSIONS[${index}]}"
  checksum="${CHECKSUMS[${index}]}"
  lookup_status=0

  printf '\n==> Reconciling %s %s (sha256: %s)\n' \
    "${crate}" "${version}" "${checksum}"

  if require_matching_registry_checksum "${crate}" "${version}" "${checksum}"; then
    printf 'Skipping %s %s: crates.io already has the staged archive checksum.\n' \
      "${crate}" "${version}"
    continue
  else
    lookup_status=$?
  fi

  if (( lookup_status != 4 )); then
    fail "could not safely determine whether ${crate} ${version} is already published"
  fi

  printf 'Publishing %s %s.\n' "${crate}" "${version}"
  set +e
  cargo publish \
    --manifest-path Cargo.toml \
    --locked \
    --registry crates-io \
    -p "${crate}"
  publish_status=$?
  set -e

  if (( publish_status != 0 )); then
    printf 'cargo publish exited %d for %s %s; polling crates.io before failing.\n' \
      "${publish_status}" "${crate}" "${version}" >&2
    if wait_for_matching_registry_checksum "${crate}" "${version}" "${checksum}"; then
      printf 'Treating %s %s as published because crates.io has the staged checksum.\n' \
        "${crate}" "${version}"
      continue
    else
      lookup_status=$?
    fi

    fail "cargo publish failed for ${crate} ${version}, and crates.io did not confirm the staged checksum (verification exit ${lookup_status})"
  fi

  wait_for_matching_registry_checksum "${crate}" "${version}" "${checksum}"
done

printf '\nAll crates are published with checksums matching the staged archives.\n'
