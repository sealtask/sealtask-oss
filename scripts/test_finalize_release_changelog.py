#!/usr/bin/env python3

from __future__ import annotations

from pathlib import Path
import sys
import tempfile
import unittest


sys.path.insert(0, str(Path(__file__).resolve().parent))

import finalize_release_changelog as subject


REPOSITORY = "https://github.com/sealtask/sealtask-oss"


class FinalizeReleaseChangelogTests(unittest.TestCase):
    def setUp(self) -> None:
        self.temporary_directory = tempfile.TemporaryDirectory()
        self.workspace = Path(self.temporary_directory.name)
        self.set_workspace_version("0.4.0")

    def tearDown(self) -> None:
        self.temporary_directory.cleanup()

    def set_workspace_version(self, version: str) -> None:
        (self.workspace / "Cargo.toml").write_text(
            f'[workspace]\n[workspace.package]\nversion = "{version}"\n',
            encoding="utf-8",
        )

    def write_changelog(self, contents: str) -> None:
        (self.workspace / "CHANGELOG.md").write_text(contents, encoding="utf-8")

    def changelog(self) -> str:
        return (self.workspace / "CHANGELOG.md").read_text(encoding="utf-8")

    def assert_finalize_fails_without_changes(self, expected_error: str) -> None:
        original = self.changelog()
        with self.assertRaisesRegex(ValueError, expected_error):
            subject.finalize(self.workspace)
        self.assertEqual(self.changelog(), original)

    def test_normalizes_release_plz_inline_heading_and_adds_reference(self) -> None:
        self.write_changelog(
            "# Changelog\n\n"
            "## [Unreleased]\n\n"
            "## [0.4.0]"
            f"({REPOSITORY}/compare/v0.3.0...v0.4.0)"
            " - 2026-08-01\n\n"
            "### Added\n\n- Release automation.\n\n"
            "## [0.3.0] - 2026-07-28\n\n"
            "### Added\n\n- Previous release.\n\n"
            f"[Unreleased]: {REPOSITORY}/compare/v0.3.0...HEAD\n"
            f"[0.3.0]: {REPOSITORY}/compare/v0.2.1...v0.3.0\n"
        )

        result = subject.finalize(self.workspace)

        self.assertEqual(result.version, "0.4.0")
        self.assertEqual(result.previous_version, "0.3.0")
        self.assertEqual(result.release_date.isoformat(), "2026-08-01")
        self.assertEqual(
            self.changelog(),
            "# Changelog\n\n"
            "## [Unreleased]\n\n"
            "## [0.4.0] - 2026-08-01\n\n"
            "### Added\n\n- Release automation.\n\n"
            "## [0.3.0] - 2026-07-28\n\n"
            "### Added\n\n- Previous release.\n\n"
            f"[Unreleased]: {REPOSITORY}/compare/v0.4.0...HEAD\n"
            f"[0.4.0]: {REPOSITORY}/compare/v0.3.0...v0.4.0\n"
            f"[0.3.0]: {REPOSITORY}/compare/v0.2.1...v0.3.0\n",
        )

    def test_is_idempotent_for_prepare_initial_release_output(self) -> None:
        self.set_workspace_version("0.3.0")
        prepared = (
            "# Changelog\n\n"
            "## [Unreleased]\n\n"
            "## [0.3.0] - 2026-07-28\n\n"
            "### Added\n\n- Initial automated release.\n\n"
            "## [0.2.1] - 2026-07-25\n\n"
            f"[Unreleased]: {REPOSITORY}/compare/v0.3.0...HEAD\n"
            f"[0.3.0]: {REPOSITORY}/compare/v0.2.1...v0.3.0\n"
            f"[0.2.1]: {REPOSITORY}/compare/v0.2.0...v0.2.1\n"
        )
        self.write_changelog(prepared)

        first = subject.finalize(self.workspace)
        second = subject.finalize(self.workspace)

        self.assertEqual(first, second)
        self.assertEqual(self.changelog(), prepared)

    def test_finalizes_canonical_heading_from_previous_release_evidence(self) -> None:
        self.write_changelog(
            "# Changelog\n\n"
            "## [Unreleased]\n\n"
            "## [0.4.0] - 2026-08-01\n\n"
            "## [0.3.0] - 2026-07-28\n\n"
            f"[Unreleased]: {REPOSITORY}/compare/v0.3.0...HEAD\n"
            f"[0.3.0]: {REPOSITORY}/compare/v0.2.1...v0.3.0\n"
        )

        subject.finalize(self.workspace)

        self.assertIn(
            f"[Unreleased]: {REPOSITORY}/compare/v0.4.0...HEAD",
            self.changelog(),
        )
        self.assertIn(
            f"[0.4.0]: {REPOSITORY}/compare/v0.3.0...v0.4.0",
            self.changelog(),
        )

    def test_rejects_duplicate_current_headings(self) -> None:
        self.write_changelog(
            "# Changelog\n\n"
            "## [Unreleased]\n\n"
            "## [0.4.0] - 2026-08-01\n\n"
            "## [0.4.0] - 2026-08-01\n\n"
            "## [0.3.0] - 2026-07-28\n\n"
            f"[Unreleased]: {REPOSITORY}/compare/v0.3.0...HEAD\n"
        )

        self.assert_finalize_fails_without_changes(
            "duplicate changelog heading for version 0.4.0"
        )

    def test_rejects_duplicate_unreleased_definitions(self) -> None:
        self.write_changelog(
            "# Changelog\n\n"
            "## [Unreleased]\n\n"
            "## [0.4.0] - 2026-08-01\n\n"
            "## [0.3.0] - 2026-07-28\n\n"
            f"[Unreleased]: {REPOSITORY}/compare/v0.3.0...HEAD\n"
            f"[Unreleased]: {REPOSITORY}/compare/v0.3.0...HEAD\n"
        )

        self.assert_finalize_fails_without_changes(
            r"duplicate changelog link definition \[Unreleased\]"
        )

    def test_rejects_malformed_current_heading_and_reference(self) -> None:
        self.write_changelog(
            "# Changelog\n\n"
            "## [Unreleased]\n\n"
            "## [0.4.0](not-a-comparison) - 2026-08-01\n\n"
            "## [0.3.0] - 2026-07-28\n\n"
            f"[Unreleased]: {REPOSITORY}/compare/v0.3.0...HEAD\n"
        )
        self.assert_finalize_fails_without_changes("inline heading for 0.4.0 must use")

        self.write_changelog(
            "# Changelog\n\n"
            "## [Unreleased]\n\n"
            "## [0.4.0] - 2026-08-01\n\n"
            "## [0.3.0] - 2026-07-28\n\n"
            f"[Unreleased]: {REPOSITORY}/compare/v0.3.0...HEAD\n"
            "[0.4.0] = malformed\n"
        )
        self.assert_finalize_fails_without_changes(
            r"malformed \[0.4.0\] link definition"
        )

    def test_rejects_inconsistent_previous_versions(self) -> None:
        self.write_changelog(
            "# Changelog\n\n"
            "## [Unreleased]\n\n"
            "## [0.4.0]"
            f"({REPOSITORY}/compare/v0.3.0...v0.4.0)"
            " - 2026-08-01\n\n"
            "## [0.2.1] - 2026-07-25\n\n"
            f"[Unreleased]: {REPOSITORY}/compare/v0.3.0...HEAD\n"
        )

        self.assert_finalize_fails_without_changes(
            "inconsistent previous release versions"
        )

    def test_rejects_wrong_reference_target_and_nonpreceding_version(self) -> None:
        self.write_changelog(
            "# Changelog\n\n"
            "## [Unreleased]\n\n"
            "## [0.4.0] - 2026-08-01\n\n"
            "## [0.3.0] - 2026-07-28\n\n"
            f"[Unreleased]: {REPOSITORY}/compare/v0.4.0...HEAD\n"
            f"[0.4.0]: {REPOSITORY}/compare/v0.3.0...v9.9.9\n"
        )
        self.assert_finalize_fails_without_changes(
            r"\[0.4.0\] link definition compares to '9.9.9'"
        )

        self.write_changelog(
            "# Changelog\n\n"
            "## [Unreleased]\n\n"
            "## [0.4.0] - 2026-08-01\n\n"
            "## [0.5.0] - 2026-07-28\n\n"
            f"[Unreleased]: {REPOSITORY}/compare/v0.5.0...HEAD\n"
        )
        self.assert_finalize_fails_without_changes(
            "previous release 0.5.0 must precede current release 0.4.0"
        )


if __name__ == "__main__":
    unittest.main()
