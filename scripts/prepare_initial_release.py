#!/usr/bin/env python3
"""Prepare the one-time 0.3.0 release that bootstraps automated releases."""

from __future__ import annotations

import argparse
import datetime as dt
from pathlib import Path
import re
import sys
import tomllib


FROM_VERSION = "0.2.1"
TO_VERSION = "0.3.0"
INTERNAL_MANIFESTS = (
    Path("cli/Cargo.toml"),
    Path("crates/client-auth/Cargo.toml"),
    Path("crates/client-crypto/Cargo.toml"),
    Path("crates/client-api/Cargo.toml"),
    Path("crates/client-runtime/Cargo.toml"),
)


def replace_once(contents: str, old: str, new: str, description: str) -> str:
    count = contents.count(old)
    if count != 1:
        raise ValueError(f"expected one {description}, found {count}")
    return contents.replace(old, new, 1)


def prepare(workspace: Path, release_date: dt.date) -> None:
    root_manifest = workspace / "Cargo.toml"
    root_contents = root_manifest.read_text(encoding="utf-8")
    metadata = tomllib.loads(root_contents)
    current = metadata["workspace"]["package"]["version"]
    if current != FROM_VERSION:
        raise ValueError(
            f"initial release expects workspace version {FROM_VERSION}, found {current}"
        )

    root_contents = replace_once(
        root_contents,
        f'version = "{FROM_VERSION}"',
        f'version = "{TO_VERSION}"',
        "workspace version",
    )
    root_manifest.write_text(root_contents, encoding="utf-8")

    old_requirement = f'version = "={FROM_VERSION}"'
    new_requirement = f'version = "={TO_VERSION}"'
    replacements = 0
    for relative_manifest in INTERNAL_MANIFESTS:
        manifest = workspace / relative_manifest
        contents = manifest.read_text(encoding="utf-8")
        count = contents.count(old_requirement)
        contents = contents.replace(old_requirement, new_requirement)
        manifest.write_text(contents, encoding="utf-8")
        replacements += count
    if replacements != 13:
        raise ValueError(
            f"expected 13 exact internal dependency requirements, found {replacements}"
        )

    changelog = workspace / "CHANGELOG.md"
    changelog_contents = changelog.read_text(encoding="utf-8")
    unreleased_marker = "## [Unreleased]\n\n"
    release_marker = (
        "## [Unreleased]\n\n"
        f"## [{TO_VERSION}] - {release_date.isoformat()}\n\n"
    )
    changelog_contents = replace_once(
        changelog_contents,
        unreleased_marker,
        release_marker,
        "Unreleased changelog heading",
    )
    changelog_contents = replace_once(
        changelog_contents,
        (
            f"[Unreleased]: https://github.com/sealtask/sealtask-oss/"
            f"compare/v{FROM_VERSION}...HEAD\n"
        ),
        (
            f"[Unreleased]: https://github.com/sealtask/sealtask-oss/"
            f"compare/v{TO_VERSION}...HEAD\n"
            f"[{TO_VERSION}]: https://github.com/sealtask/sealtask-oss/"
            f"compare/v{FROM_VERSION}...v{TO_VERSION}\n"
        ),
        "Unreleased changelog link",
    )
    changelog.write_text(changelog_contents, encoding="utf-8")


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument(
        "--workspace",
        type=Path,
        default=Path(__file__).resolve().parent.parent,
    )
    parser.add_argument("--date", required=True)
    args = parser.parse_args()
    try:
        release_date = dt.date.fromisoformat(args.date)
        prepare(args.workspace.resolve(), release_date)
    except (KeyError, OSError, ValueError, tomllib.TOMLDecodeError) as error:
        print(f"error: {error}", file=sys.stderr)
        return 1
    print(f"Prepared initial OSS release {TO_VERSION} ({release_date.isoformat()}).")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
