#!/usr/bin/env bash
set -euo pipefail

OSS_DIR="$(cd "$(dirname "$0")/.." && pwd)"
WASM_PATH="${1:-$OSS_DIR/artifacts/strong-box-wasm/strong_box_wasm_bg.wasm}"

if [[ ! -f "$WASM_PATH" ]]; then
  echo "StrongBox WASM artifact not found at $WASM_PATH" >&2
  exit 1
fi

if ! command -v strings >/dev/null 2>&1; then
  echo "strings is required to check StrongBox WASM debug paths" >&2
  exit 1
fi

LEAK_PATTERN='(^/.*(registry/src|rustlib/src)|^[A-Za-z]:[\\/].*(registry[\\/]src|rustlib[\\/]src)|/Users/|/home/|/private/|\.cargo|\.rustup|rustup/toolchains|[A-Za-z0-9_+.-]+-(apple-darwin|pc-windows-(gnu|msvc)|unknown-linux-(gnu|musl)))'
matches="$(strings "$WASM_PATH" | grep -E "$LEAK_PATTERN" || true)"

if [[ -n "$matches" ]]; then
  echo "StrongBox WASM contains host-specific build paths or toolchain triples:" >&2
  printf '%s\n' "$matches" | head -n 20 >&2
  exit 1
fi

echo "StrongBox WASM path check passed."
