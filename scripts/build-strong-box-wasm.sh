#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
OSS_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
MODE="${1:-build}"

case "$MODE" in
  build | update | verify)
    if [[ $# -gt 0 ]]; then
      shift
    fi
    ;;
  *)
    echo "usage: $0 {build|update|verify} [--output PATH]" >&2
    exit 2
    ;;
esac

OUTPUT_PATH=""
while [[ $# -gt 0 ]]; do
  case "$1" in
    --output)
      if [[ $# -lt 2 ]]; then
        echo "--output requires a path" >&2
        exit 2
      fi
      OUTPUT_PATH="$2"
      shift 2
      ;;
    *)
      echo "unknown argument: $1" >&2
      exit 2
      ;;
  esac
done

if ! command -v cargo >/dev/null 2>&1; then
  echo "cargo is required to build the StrongBox WASM bridge" >&2
  exit 1
fi

if [[ "$MODE" != "build" ]]; then
  if ! command -v python3 >/dev/null 2>&1; then
    echo "Python 3.11 or newer is required to verify the WASM manifest" >&2
    exit 1
  fi

  python3 -c 'import sys; raise SystemExit(sys.version_info < (3, 11))' || {
    echo "Python 3.11 or newer is required to verify the WASM manifest" >&2
    exit 1
  }
fi

canonical_dir() {
  local path="$1"
  if [[ -d "$path" ]]; then
    (cd "$path" && pwd -P)
  else
    printf '%s\n' "$path"
  fi
}

absolute_path() {
  local path="$1"
  case "$path" in
    /*)
      printf '%s\n' "$path"
      ;;
    *)
      printf '%s/%s\n' "$OSS_DIR" "$path"
      ;;
  esac
}

append_encoded_flag() {
  if [[ -n "$ENCODED_RUSTFLAGS" ]]; then
    ENCODED_RUSTFLAGS+="$UNIT_SEPARATOR"
  fi
  ENCODED_RUSTFLAGS+="$1"
}

require_canonical_platform() {
  local os
  local arch
  os="$(uname -s)"
  arch="$(uname -m)"
  if [[ "$os" != "Linux" || "$arch" != "x86_64" ]]; then
    echo "$MODE requires the canonical linux/amd64 build platform; got $os/$arch" >&2
    exit 1
  fi
}

TARGET_DIR="${STRONG_BOX_WASM_TARGET_DIR:-${CARGO_TARGET_DIR:-${CARGO_BUILD_TARGET_DIR:-$OSS_DIR/target}}}"
TARGET_DIR="$(absolute_path "$TARGET_DIR")"
PROFILE_DIR="$TARGET_DIR/wasm32-unknown-unknown/wasm-release"
BUILT_WASM="$PROFILE_DIR/strong_box_wasm.wasm"
DEPS_WASM="$PROFILE_DIR/deps/strong_box_wasm.wasm"
PUBLIC_ARTIFACT="$OSS_DIR/artifacts/strong-box-wasm/strong_box_wasm_bg.wasm"
PATH_CHECK="$SCRIPT_DIR/check-strong-box-wasm-paths.sh"
MANIFEST_TOOL="$SCRIPT_DIR/strong-box-wasm-manifest.py"

if [[ "$MODE" == "update" || "$MODE" == "verify" ]]; then
  require_canonical_platform
fi

RUST_VERSION="$(cd "$OSS_DIR" && rustc --version)"
case "$RUST_VERSION" in
  "rustc 1.94.0 "*)
    ;;
  *)
    echo "StrongBox WASM requires rustc 1.94.0; got $RUST_VERSION" >&2
    exit 1
    ;;
esac

RUST_SYSROOT="$(cd "$OSS_DIR" && rustc --print sysroot)"
CARGO_HOME_DIR="${CARGO_HOME:-$HOME/.cargo}"
UNIT_SEPARATOR="$(printf '\037')"
ENCODED_RUSTFLAGS=""

append_encoded_flag "--remap-path-prefix=$OSS_DIR=workspace"
append_encoded_flag "--remap-path-prefix=$(canonical_dir "$OSS_DIR")=workspace"
append_encoded_flag "--remap-path-prefix=$TARGET_DIR=workspace/target"
append_encoded_flag "--remap-path-prefix=$(canonical_dir "$TARGET_DIR")=workspace/target"
append_encoded_flag "--remap-path-prefix=$CARGO_HOME_DIR=cargo"
append_encoded_flag "--remap-path-prefix=$(canonical_dir "$CARGO_HOME_DIR")=cargo"
append_encoded_flag "--remap-path-prefix=$RUST_SYSROOT=rust"
append_encoded_flag "--remap-path-prefix=$(canonical_dir "$RUST_SYSROOT")=rust"

mkdir -p "$PROFILE_DIR/deps"
rm -f "$BUILT_WASM" "$DEPS_WASM"

echo "Building StrongBox WASM with rustc 1.94.0 and profile wasm-release..."
(
  cd "$OSS_DIR"
  CARGO_TARGET_DIR="$TARGET_DIR" \
    CARGO_INCREMENTAL=0 \
    CARGO_ENCODED_RUSTFLAGS="$ENCODED_RUSTFLAGS" \
    RUSTFLAGS= \
    SOURCE_DATE_EPOCH=0 \
    cargo build \
      -p strong-box-wasm \
      --profile wasm-release \
      --locked \
      --target wasm32-unknown-unknown
)

if [[ ! -f "$BUILT_WASM" ]]; then
  echo "expected WASM artifact not found at $BUILT_WASM" >&2
  exit 1
fi

"$PATH_CHECK" "$BUILT_WASM"

case "$MODE" in
  build)
    if [[ -n "$OUTPUT_PATH" ]]; then
      mkdir -p "$(dirname "$OUTPUT_PATH")"
      cp "$BUILT_WASM" "$OUTPUT_PATH"
      "$PATH_CHECK" "$OUTPUT_PATH"
      echo "Copied StrongBox WASM to $OUTPUT_PATH"
    fi
    ;;
  update)
    mkdir -p "$(dirname "$PUBLIC_ARTIFACT")"
    cp "$BUILT_WASM" "$PUBLIC_ARTIFACT"
    "$MANIFEST_TOOL" update
    if [[ -n "$OUTPUT_PATH" ]]; then
      mkdir -p "$(dirname "$OUTPUT_PATH")"
      cp "$BUILT_WASM" "$OUTPUT_PATH"
    fi
    ;;
  verify)
    VERIFY_ARGS=(verify --built-wasm "$BUILT_WASM")
    if [[ -n "$OUTPUT_PATH" ]]; then
      VERIFY_ARGS+=(--frontend-wasm "$OUTPUT_PATH")
    fi
    "$MANIFEST_TOOL" "${VERIFY_ARGS[@]}"
    "$PATH_CHECK" "$PUBLIC_ARTIFACT"
    if [[ -n "$OUTPUT_PATH" ]]; then
      "$PATH_CHECK" "$OUTPUT_PATH"
    fi
    ;;
esac

if command -v sha256sum >/dev/null 2>&1; then
  WASM_SHA256="$(sha256sum "$BUILT_WASM" | awk '{print $1}')"
else
  WASM_SHA256="$(shasum -a 256 "$BUILT_WASM" | awk '{print $1}')"
fi
echo "StrongBox WASM sha256: $WASM_SHA256"
