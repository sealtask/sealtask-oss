#!/usr/bin/env python3

from __future__ import annotations

import datetime as dt
from pathlib import Path
import sys
import tempfile
import unittest

sys.path.insert(0, str(Path(__file__).resolve().parent))

import prepare_initial_release as subject


class PrepareInitialReleaseTests(unittest.TestCase):
    def setUp(self) -> None:
        self.temporary_directory = tempfile.TemporaryDirectory()
        self.workspace = Path(self.temporary_directory.name)
        (self.workspace / "Cargo.toml").write_text(
            '[workspace]\n[workspace.package]\nversion = "0.2.1"\n',
            encoding="utf-8",
        )
        for manifest in subject.INTERNAL_MANIFESTS:
            path = self.workspace / manifest
            path.parent.mkdir(parents=True, exist_ok=True)
            edge_count = {
                Path("cli/Cargo.toml"): 5,
                Path("crates/client-auth/Cargo.toml"): 1,
                Path("crates/client-crypto/Cargo.toml"): 1,
                Path("crates/client-api/Cargo.toml"): 2,
                Path("crates/client-runtime/Cargo.toml"): 4,
            }[manifest]
            path.write_text(
                "\n".join(
                    f'dependency-{index} = {{ version = "=0.2.1", path = "x" }}'
                    for index in range(edge_count)
                )
                + "\n",
                encoding="utf-8",
            )
        (self.workspace / "CHANGELOG.md").write_text(
            "# Changelog\n\n"
            "## [Unreleased]\n\n"
            "### Added\n\n- A change.\n\n"
            "[Unreleased]: https://github.com/sealtask/sealtask-oss/"
            "compare/v0.2.1...HEAD\n",
            encoding="utf-8",
        )

    def tearDown(self) -> None:
        self.temporary_directory.cleanup()

    def test_prepares_the_bootstrap_release_once(self) -> None:
        subject.prepare(self.workspace, dt.date(2026, 7, 28))

        self.assertIn(
            'version = "0.3.0"',
            (self.workspace / "Cargo.toml").read_text(encoding="utf-8"),
        )
        pins = sum(
            (self.workspace / manifest)
            .read_text(encoding="utf-8")
            .count('version = "=0.3.0"')
            for manifest in subject.INTERNAL_MANIFESTS
        )
        self.assertEqual(pins, 13)
        changelog = (self.workspace / "CHANGELOG.md").read_text(encoding="utf-8")
        self.assertIn("## [0.3.0] - 2026-07-28", changelog)
        self.assertIn("compare/v0.2.1...v0.3.0", changelog)

        with self.assertRaisesRegex(ValueError, "expects workspace version"):
            subject.prepare(self.workspace, dt.date(2026, 7, 28))


if __name__ == "__main__":
    unittest.main()
