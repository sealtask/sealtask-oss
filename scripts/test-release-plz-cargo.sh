#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ADAPTER="${SCRIPT_DIR}/release-plz-cargo.sh"
TEST_DIR="$(mktemp -d "${TMPDIR:-/tmp}/sealtask-release-plz-cargo.XXXXXX")"

cleanup() {
  case "${TEST_DIR}" in
    "${TMPDIR:-/tmp}"/sealtask-release-plz-cargo.*)
      rm -rf -- "${TEST_DIR}"
      ;;
    *)
      printf 'Refusing to clean unexpected test path: %s\n' "${TEST_DIR}" >&2
      ;;
  esac
}
trap cleanup EXIT

MOCK_CARGO="${TEST_DIR}/cargo"
INVOCATIONS="${TEST_DIR}/invocations"
cat >"${MOCK_CARGO}" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
printf '%s\n' "$*" >>"${MOCK_CARGO_INVOCATIONS}"
EOF
chmod +x "${MOCK_CARGO}"

run_adapter() {
  PATH="${TEST_DIR}:${PATH}" \
  MOCK_CARGO_INVOCATIONS="${INVOCATIONS}" \
    "${ADAPTER}" "$@"
}

run_adapter metadata --format-version 1
run_adapter package --allow-dirty --workspace
run_adapter package --locked -p sealtask

expected="$(
  cat <<'EOF'
metadata --format-version 1
package --allow-dirty --workspace --exclude strong-box --exclude strong-box-wasm
package --locked -p sealtask
EOF
)"
if [[ "$(cat "${INVOCATIONS}")" != "${expected}" ]]; then
  echo "release-plz Cargo adapter forwarded unexpected arguments" >&2
  diff -u <(printf '%s\n' "${expected}") "${INVOCATIONS}" >&2 || true
  exit 1
fi

if run_adapter package --workspace --allow-dirty \
  >"${TEST_DIR}/unexpected.stdout" 2>"${TEST_DIR}/unexpected.stderr"; then
  echo "adapter accepted a changed whole-workspace package command" >&2
  exit 1
fi
grep -Fq "unexpected whole-workspace package command" \
  "${TEST_DIR}/unexpected.stderr"

mkdir -p "${TEST_DIR}/missing-bin"
if PATH="${TEST_DIR}/missing-bin" /bin/bash "${ADAPTER}" metadata \
  >"${TEST_DIR}/missing.stdout" 2>"${TEST_DIR}/missing.stderr"; then
  echo "adapter accepted a missing real Cargo executable" >&2
  exit 1
fi
grep -Fq "Cargo executable was not found" "${TEST_DIR}/missing.stderr"

echo "release-plz Cargo adapter tests passed."
