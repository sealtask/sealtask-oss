from __future__ import annotations

import importlib.util
import tempfile
import unittest
from pathlib import Path
from unittest import mock


SCRIPT_PATH = Path(__file__).with_name("strong-box-wasm-manifest.py")
SPEC = importlib.util.spec_from_file_location("strong_box_wasm_manifest", SCRIPT_PATH)
if SPEC is None or SPEC.loader is None:
    raise RuntimeError(f"could not load {SCRIPT_PATH}")
manifest = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(manifest)


class StrongBoxWasmManifestTests(unittest.TestCase):
    def test_expected_manifest_reads_the_canonical_rust_toolchain(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            oss_dir = Path(directory)
            (oss_dir / "rust-toolchain.toml").write_text(
                '[toolchain]\nchannel = "9.8.7"\n',
                encoding="utf-8",
            )
            (oss_dir / "Cargo.lock").write_text("lockfile", encoding="utf-8")
            artifact = oss_dir / "strong_box_wasm_bg.wasm"
            artifact.write_bytes(b"w" * manifest.MINIMUM_WASM_SIZE)

            with (
                mock.patch.object(manifest, "OSS_DIR", oss_dir),
                mock.patch.object(manifest, "sha256", return_value="digest"),
                mock.patch.object(
                    manifest,
                    "package_metadata",
                    return_value={
                        "name": "package",
                        "version": "1.0.0",
                        "path": "crates/package",
                    },
                ),
                mock.patch.object(
                    manifest,
                    "tool_version",
                    side_effect=["rustc 9.8.7 (example)", "cargo 9.8.7 (example)"],
                ),
            ):
                result = manifest.expected_manifest(artifact)

        self.assertEqual(result["build"]["rustToolchain"], "9.8.7")
        self.assertEqual(result["build"]["rustcVersion"], "rustc 9.8.7 (example)")

    def test_expected_manifest_rejects_a_compiler_that_does_not_match_the_pin(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            oss_dir = Path(directory)
            (oss_dir / "rust-toolchain.toml").write_text(
                '[toolchain]\nchannel = "9.8.7"\n',
                encoding="utf-8",
            )
            (oss_dir / "Cargo.lock").write_text("lockfile", encoding="utf-8")
            artifact = oss_dir / "strong_box_wasm_bg.wasm"
            artifact.write_bytes(b"w" * manifest.MINIMUM_WASM_SIZE)

            with (
                mock.patch.object(manifest, "OSS_DIR", oss_dir),
                mock.patch.object(manifest, "sha256", return_value="digest"),
                mock.patch.object(
                    manifest,
                    "tool_version",
                    return_value="rustc 9.8.8 (example)",
                ),
            ):
                with self.assertRaisesRegex(ValueError, "active compiler"):
                    manifest.expected_manifest(artifact)


if __name__ == "__main__":
    unittest.main()
