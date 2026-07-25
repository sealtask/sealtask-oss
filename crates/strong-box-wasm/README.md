# StrongBox WASM bridge

This GPL-3.0-only crate is the Rust source for the StrongBox WebAssembly byte
shipped by SealTask. It exposes a small C ABI for allocation, symmetric
encryption/decryption, and the X25519/HKDF-SHA256/ChaCha20-Poly1305 operations
used by browser HPKE flows.

Build and provenance verification are owned by the workspace-level script:

```bash
./scripts/build-strong-box-wasm.sh build
./scripts/build-strong-box-wasm.sh verify
```

The bridge imports browser-provided randomness through the
`strong_box_random` host function. See the workspace README and
`artifacts/strong-box-wasm/build-manifest.json` for the pinned toolchain,
profile, and artifact digest.
