#!/usr/bin/env python3
"""Generate and strictly verify the StrongBox WASM build manifest."""

from __future__ import annotations

import argparse
import hashlib
import json
import subprocess
import sys
import tomllib
from pathlib import Path
from typing import Any

OSS_DIR = Path(__file__).resolve().parent.parent
ARTIFACT_RELATIVE_PATH = Path(
    "artifacts/strong-box-wasm/strong_box_wasm_bg.wasm"
)
MANIFEST_RELATIVE_PATH = Path("artifacts/strong-box-wasm/build-manifest.json")
CARGO_LOCK_RELATIVE_PATH = Path("Cargo.lock")
REPOSITORY = "https://github.com/sealtask/sealtask-oss"
MINIMUM_WASM_SIZE = 1024


def sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as file:
        for chunk in iter(lambda: file.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def require_file(path: Path, label: str) -> None:
    if not path.is_file():
        raise ValueError(f"{label} not found at {path}")


def package_metadata(relative_manifest: str) -> dict[str, str]:
    manifest_path = OSS_DIR / relative_manifest
    with manifest_path.open("rb") as file:
        package = tomllib.load(file)["package"]

    return {
        "name": str(package["name"]),
        "version": str(package["version"]),
        "path": str(Path(relative_manifest).parent),
    }


def tool_version(command: str) -> str:
    return subprocess.check_output(
        [command, "--version"],
        cwd=OSS_DIR,
        text=True,
    ).strip()


def expected_manifest(artifact: Path) -> dict[str, Any]:
    cargo_lock = OSS_DIR / CARGO_LOCK_RELATIVE_PATH
    require_file(artifact, "StrongBox WASM artifact")
    require_file(cargo_lock, "OSS Cargo lockfile")

    artifact_size = artifact.stat().st_size
    if artifact_size < MINIMUM_WASM_SIZE:
        raise ValueError(
            f"StrongBox WASM artifact is unexpectedly small: {artifact_size} bytes"
        )

    return {
        "schemaVersion": 1,
        "artifact": {
            "path": ARTIFACT_RELATIVE_PATH.as_posix(),
            "sha256": sha256(artifact),
            "sizeBytes": artifact_size,
        },
        "source": {
            "repository": REPOSITORY,
            "cargoLock": {
                "path": CARGO_LOCK_RELATIVE_PATH.as_posix(),
                "sha256": sha256(cargo_lock),
            },
            "packages": [
                package_metadata("crates/strong-box/Cargo.toml"),
                package_metadata("crates/strong-box-wasm/Cargo.toml"),
            ],
        },
        "build": {
            "rustToolchain": "1.94.0",
            "rustcVersion": tool_version("rustc"),
            "cargoVersion": tool_version("cargo"),
            "target": "wasm32-unknown-unknown",
            "profile": "wasm-release",
            "canonicalPlatform": "linux/amd64",
            "command": "./scripts/build-strong-box-wasm.sh update",
            "wasmOpt": None,
        },
        "license": "GPL-3.0-only",
    }


def write_manifest() -> None:
    artifact = OSS_DIR / ARTIFACT_RELATIVE_PATH
    manifest_path = OSS_DIR / MANIFEST_RELATIVE_PATH
    manifest_path.parent.mkdir(parents=True, exist_ok=True)
    manifest_path.write_text(
        json.dumps(expected_manifest(artifact), indent=2) + "\n",
        encoding="utf-8",
    )
    print(f"Wrote {manifest_path}")


def load_manifest() -> dict[str, Any]:
    manifest_path = OSS_DIR / MANIFEST_RELATIVE_PATH
    require_file(manifest_path, "StrongBox WASM build manifest")
    try:
        value = json.loads(manifest_path.read_text(encoding="utf-8"))
    except json.JSONDecodeError as error:
        raise ValueError(f"invalid JSON in {manifest_path}: {error}") from error
    if not isinstance(value, dict):
        raise ValueError(f"{manifest_path} must contain a JSON object")
    return value


def verify_manifest(built_wasm: Path, frontend_wasm: Path | None) -> None:
    artifact = OSS_DIR / ARTIFACT_RELATIVE_PATH
    require_file(built_wasm, "rebuilt StrongBox WASM")
    require_file(artifact, "committed StrongBox WASM artifact")

    built_hash = sha256(built_wasm)
    artifact_hash = sha256(artifact)
    if built_hash != artifact_hash:
        raise ValueError(
            "rebuilt StrongBox WASM does not match the committed artifact: "
            f"{built_hash} != {artifact_hash}"
        )

    if frontend_wasm is not None:
        require_file(frontend_wasm, "frontend StrongBox WASM artifact")
        frontend_hash = sha256(frontend_wasm)
        if frontend_hash != built_hash:
            raise ValueError(
                "frontend StrongBox WASM does not match the canonical rebuild: "
                f"{frontend_hash} != {built_hash}"
            )

    expected = expected_manifest(artifact)
    actual = load_manifest()
    if actual != expected:
        raise ValueError(
            "StrongBox WASM build manifest is stale or has unexpected fields; "
            "run ./scripts/build-strong-box-wasm.sh update on linux/amd64"
        )

    print(
        "Verified StrongBox WASM artifact and build manifest "
        f"(sha256: {built_hash})"
    )


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    subparsers = parser.add_subparsers(dest="command", required=True)
    subparsers.add_parser("update")

    verify = subparsers.add_parser("verify")
    verify.add_argument("--built-wasm", type=Path, required=True)
    verify.add_argument("--frontend-wasm", type=Path)
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    try:
        if args.command == "update":
            write_manifest()
        else:
            verify_manifest(
                args.built_wasm.resolve(),
                args.frontend_wasm.resolve() if args.frontend_wasm else None,
            )
    except (OSError, KeyError, TypeError, ValueError, subprocess.CalledProcessError) as error:
        print(f"error: {error}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
