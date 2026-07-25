import { defineConfig, type PluginOption } from 'vite'

const argon2WasmPattern =
  /argon2-browser\/dist\/argon2(?:-simd)?\.wasm$/

function shimArgon2WasmImport(): PluginOption {
  return {
    name: 'crypto-web-browser-test-shim-argon2-wasm',
    load(id) {
      if (argon2WasmPattern.test(id.replace(/\\/g, '/'))) {
        // argon2-browser's CommonJS branch requires the WASM file as a module.
        // The runtime installs a URL-backed binary loader before hashing, so
        // this legacy require must not be treated as an ESM WASM import by Vite.
        return 'export default ""'
      }
    },
  }
}

export default defineConfig({
  plugins: [shimArgon2WasmImport()],
})
