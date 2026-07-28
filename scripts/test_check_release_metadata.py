#!/usr/bin/env python3
"""Offline regression tests for check_release_metadata.py."""

from __future__ import annotations

import json
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path
from unittest import mock

sys.path.insert(0, str(Path(__file__).resolve().parent))

import check_release_metadata as checker


VERSION = "1.2.3"

DEPENDENCIES = {
    "sealtask-client-core": (),
    "sealtask-client-auth": ("sealtask-client-core",),
    "sealtask-client-crypto": ("sealtask-client-core",),
    "sealtask-client-api": (
        "sealtask-client-auth",
        "sealtask-client-core",
    ),
    "sealtask-client-runtime": (
        "sealtask-client-api",
        "sealtask-client-auth",
        "sealtask-client-core",
        "sealtask-client-crypto",
    ),
    "sealtask": (
        "sealtask-client-api",
        "sealtask-client-auth",
        "sealtask-client-core",
        "sealtask-client-crypto",
        "sealtask-client-runtime",
    ),
}


class ReleaseMetadataFixture:
    def __init__(self, root: Path) -> None:
        self.root = root
        self.manifest_path = root / "Cargo.toml"
        self.manifest_path.write_text("[workspace]\n", encoding="utf-8")

        man_directory = root / "cli/assets/man"
        man_directory.mkdir(parents=True)
        for name in ("sealtask.1", "sealtask-info.1"):
            (man_directory / name).write_text(
                f'.TH sealtask 1  "SealTask CLI {VERSION}" "SealTask Manual"\n',
                encoding="utf-8",
            )

        (root / "CHANGELOG.md").write_text(
            f"# Changelog\n\n## [{VERSION}] - 2026-07-28\n",
            encoding="utf-8",
        )

        packages = [
            self._publishable_package(name) for name in checker.PUBLISH_ORDER
        ]
        packages.extend(
            (
                self._unpublished_package(
                    "strong-box", "crates/strong-box/Cargo.toml", "0.0.0-git"
                ),
                self._unpublished_package(
                    "strong-box-wasm",
                    "crates/strong-box-wasm/Cargo.toml",
                    "0.1.0",
                ),
            )
        )
        self.metadata = {
            "workspace_root": str(root),
            "workspace_members": [package["id"] for package in packages],
            "packages": packages,
        }

    def _publishable_package(self, name: str) -> dict[str, object]:
        manifest_path = self.root / checker.EXPECTED_MANIFESTS[name]
        manifest_path.parent.mkdir(parents=True, exist_ok=True)
        manifest_path.touch()
        dependencies = [
            {
                "name": dependency_name,
                "req": f"={VERSION}",
                "path": str(
                    self.root / checker.EXPECTED_MANIFESTS[dependency_name].parent
                ),
                "kind": None,
                "target": None,
            }
            for dependency_name in DEPENDENCIES[name]
        ]
        return {
            "id": f"path+file://{manifest_path.parent}#{name}@{VERSION}",
            "name": name,
            "version": VERSION,
            "manifest_path": str(manifest_path),
            "publish": ["crates-io"],
            "dependencies": dependencies,
        }

    def _unpublished_package(
        self, name: str, relative_manifest: str, version: str
    ) -> dict[str, object]:
        manifest_path = self.root / relative_manifest
        manifest_path.parent.mkdir(parents=True, exist_ok=True)
        manifest_path.touch()
        return {
            "id": f"path+file://{manifest_path.parent}#{name}@{version}",
            "name": name,
            "version": version,
            "manifest_path": str(manifest_path),
            "publish": [],
            "dependencies": [],
        }

    def package(self, name: str) -> dict[str, object]:
        return next(
            package
            for package in self.metadata["packages"]
            if package["name"] == name
        )


class ReleaseMetadataValidationTests(unittest.TestCase):
    def setUp(self) -> None:
        self.temporary_directory = tempfile.TemporaryDirectory()
        self.root = Path(self.temporary_directory.name)
        self.fixture = ReleaseMetadataFixture(self.root)

    def tearDown(self) -> None:
        self.temporary_directory.cleanup()

    def validate(self, mode: str = "workspace", tag: str | None = None):
        return checker.validate_release_metadata(
            self.fixture.metadata,
            self.fixture.manifest_path,
            mode,
            tag,
        )

    def assert_failure(self, expected: str, mode: str = "workspace", tag=None) -> None:
        with self.assertRaises(checker.ValidationFailure) as context:
            self.validate(mode, tag)
        self.assertEqual(context.exception.messages, (expected,))

    def test_workspace_mode_accepts_complete_offline_metadata(self) -> None:
        report = self.validate()

        self.assertEqual(report.version, VERSION)
        self.assertEqual(report.man_page_count, 2)

    def test_release_mode_accepts_matching_tag_and_changelog(self) -> None:
        report = self.validate("release", f"v{VERSION}")

        self.assertEqual(report.version, VERSION)

    def test_publishable_workspace_crates_are_an_exact_allowlist(self) -> None:
        self.fixture.package("strong-box")["publish"] = None

        self.assert_failure(
            "publishable workspace crates must be exactly "
            "[sealtask, sealtask-client-api, sealtask-client-auth, "
            "sealtask-client-core, sealtask-client-crypto, "
            "sealtask-client-runtime]; "
            "found [sealtask, sealtask-client-api, sealtask-client-auth, "
            "sealtask-client-core, sealtask-client-crypto, "
            "sealtask-client-runtime, strong-box]"
        )

    def test_each_release_crate_is_limited_to_crates_io(self) -> None:
        self.fixture.package("sealtask-client-core")["publish"] = ["private"]

        self.assert_failure(
            'sealtask-client-core must set publish = ["crates-io"]; '
            'cargo metadata reported ["private"]'
        )

    def test_release_crates_must_share_one_version(self) -> None:
        self.fixture.package("sealtask-client-api")["version"] = "1.2.4"

        self.assert_failure(
            "publishable crates must share one workspace version; found "
            "sealtask-client-core=1.2.3, sealtask-client-auth=1.2.3, "
            "sealtask-client-crypto=1.2.3, sealtask-client-api=1.2.4, "
            "sealtask-client-runtime=1.2.3, sealtask=1.2.3"
        )

    def test_workspace_version_must_be_strict_semver(self) -> None:
        for package_name in checker.PUBLISH_ORDER:
            self.fixture.package(package_name)["version"] = "01.2.3"

        self.assert_failure("workspace version '01.2.3' is not valid SemVer 2.0")

    def test_internal_dependency_requirement_must_be_exact(self) -> None:
        dependency = self.fixture.package("sealtask-client-auth")["dependencies"][0]
        dependency["req"] = "^1.2.3"

        self.assert_failure(
            "sealtask-client-auth dependency sealtask-client-core "
            "must require '=1.2.3'; found '^1.2.3'"
        )

    def test_internal_dependency_path_must_be_canonical(self) -> None:
        dependency = self.fixture.package("sealtask-client-auth")["dependencies"][0]
        dependency["path"] = str(self.root / "wrong/core")

        self.assert_failure(
            "sealtask-client-auth dependency sealtask-client-core "
            "must use path 'crates/client-core'; found 'wrong/core'"
        )

    def test_publication_order_must_be_topological(self) -> None:
        core_package = self.fixture.package("sealtask-client-core")
        core_package["dependencies"] = [
            {
                "name": "sealtask-client-api",
                "req": f"={VERSION}",
                "path": str(self.root / "crates/client-api"),
                "kind": None,
                "target": None,
            }
        ]

        self.assert_failure(
            "publication order is not topological: "
            "sealtask-client-core depends on later crate sealtask-client-api"
        )

    def test_all_generated_man_pages_must_match_the_workspace_version(self) -> None:
        (self.root / "cli/assets/man/sealtask-info.1").write_text(
            '.TH sealtask-info 1  "SealTask CLI 9.9.9" "SealTask Manual"\n',
            encoding="utf-8",
        )

        self.assert_failure(
            "cli/assets/man/sealtask-info.1 has CLI version '9.9.9'; "
            "expected '1.2.3'"
        )

    def test_generated_man_pages_must_exist(self) -> None:
        for path in (self.root / "cli/assets/man").glob("*.1"):
            path.unlink()

        self.assert_failure(
            "cli/assets/man must contain at least one generated .1 man page"
        )

    def test_release_tag_must_exactly_match_workspace_version(self) -> None:
        self.assert_failure(
            "release tag must be 'v1.2.3'; found 'v1.2.4'",
            "release",
            "v1.2.4",
        )

    def test_release_changelog_heading_must_exist_exactly_once(self) -> None:
        (self.root / "CHANGELOG.md").write_text(
            f"# Changelog\n\n## [{VERSION}] - 2026-07-28\n"
            f"\n## [{VERSION}] - 2026-07-27\n",
            encoding="utf-8",
        )

        self.assert_failure(
            "CHANGELOG.md must contain exactly one heading for '1.2.3'; found 2",
            "release",
            f"v{VERSION}",
        )

    def test_release_changelog_heading_uses_keep_a_changelog_format(self) -> None:
        (self.root / "CHANGELOG.md").write_text(
            f"# Changelog\n\n## [{VERSION}]\n",
            encoding="utf-8",
        )

        self.assert_failure(
            "CHANGELOG.md release heading must match "
            "'## [1.2.3] - YYYY-MM-DD'; found '## [1.2.3]'",
            "release",
            f"v{VERSION}",
        )

    def test_release_changelog_date_must_be_real(self) -> None:
        (self.root / "CHANGELOG.md").write_text(
            f"# Changelog\n\n## [{VERSION}] - 2026-02-31\n",
            encoding="utf-8",
        )

        self.assert_failure(
            "CHANGELOG.md release heading has invalid date '2026-02-31'",
            "release",
            f"v{VERSION}",
        )

    def test_workspace_mode_does_not_require_a_changelog(self) -> None:
        (self.root / "CHANGELOG.md").unlink()

        report = self.validate()

        self.assertEqual(report.version, VERSION)


class CargoMetadataCommandTests(unittest.TestCase):
    @mock.patch("check_release_metadata.subprocess.run")
    def test_metadata_command_is_locked_dependency_free_and_offline(
        self, run: mock.Mock
    ) -> None:
        run.return_value = subprocess.CompletedProcess(
            args=[], returncode=0, stdout=json.dumps({"packages": []}), stderr=""
        )
        manifest_path = Path("/tmp/oss/Cargo.toml")

        checker.load_cargo_metadata(manifest_path, "/opt/bin/cargo")

        command = run.call_args.args[0]
        self.assertEqual(
            command,
            [
                "/opt/bin/cargo",
                "metadata",
                "--manifest-path",
                str(manifest_path),
                "--format-version",
                "1",
                "--locked",
                "--no-deps",
                "--offline",
            ],
        )
        self.assertFalse(run.call_args.kwargs["check"])
        self.assertEqual(
            run.call_args.kwargs["env"]["CARGO_NET_OFFLINE"],
            "true",
        )


if __name__ == "__main__":
    unittest.main()
