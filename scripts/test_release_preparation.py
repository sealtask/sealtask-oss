#!/usr/bin/env python3
"""Exercise the pinned release-plz against a new crate after a shared tag.

Registry manifests stand in for downloaded crates, so the regression is offline
and never publishes, pushes, or uses GitHub credentials.
"""

from pathlib import Path
import os
import shutil
import subprocess
import tempfile
import tomllib
import unittest


WORKSPACE = Path(__file__).resolve().parent.parent
PACKAGES = (
    "sealtask-client-core",
    "sealtask-client-auth",
    "sealtask-client-crypto",
    "sealtask-client-api",
    "sealtask-client-runtime",
    "sealtask",
    "sealtask-agent",
)


class ReleasePreparationTests(unittest.TestCase):
    def test_new_crate_joins_lockstep_release_after_shared_tag(self):
        release_plz = shutil.which(os.environ.get("RELEASE_PLZ_BIN", "release-plz"))
        self.assertIsNotNone(release_plz, "install pinned release-plz before running this test")
        with tempfile.TemporaryDirectory(prefix="sealtask-release-preparation-") as directory:
            root = Path(directory)
            repo = root / "repo"
            registry = root / "registry"
            env = {**os.environ, "CARGO_NET_OFFLINE": "true"}

            def run(*args, cwd=repo):
                result = subprocess.run(
                    args, cwd=cwd, env=env, text=True,
                    stdout=subprocess.PIPE, stderr=subprocess.STDOUT, timeout=120,
                )
                self.assertEqual(result.returncode, 0, result.stdout)
                return result.stdout

            def workspace(path, packages, version):
                path.mkdir(exist_ok=True)
                (path / "Cargo.toml").write_text(
                    f'[workspace]\nmembers = {list(packages)!r}\nresolver = "2"\n'
                    f'[workspace.package]\nversion = "{version}"\n',
                    encoding="utf-8",
                )
                for name in packages:
                    package = path / name
                    (package / "src").mkdir(parents=True, exist_ok=True)
                    (package / "Cargo.toml").write_text(
                        f'[package]\nname = "{name}"\nversion.workspace = true\n'
                        'edition = "2024"\nlicense = "GPL-3.0-only"\n'
                        'description = "Release regression fixture"\npublish = ["crates-io"]\n'
                        + (f'[dependencies]\nsealtask-client-core = {{ version = "={version}", '
                           'path = "../sealtask-client-core" }\n'
                           if name != "sealtask-client-core" else ""),
                        encoding="utf-8",
                    )
                    (package / "src/lib.rs").write_text("pub fn fixture() {}\n", encoding="utf-8")

            workspace(repo, PACKAGES[:-1], "0.4.0")
            workspace(registry, PACKAGES[:-1], "0.4.0")
            # Use the production configuration, disabling only API compilation
            # for these synthetic crates. Keep baseline selection and grouping.
            config = (WORKSPACE / "release-plz.toml").read_text(encoding="utf-8")
            (repo / "release-plz.toml").write_text(
                config.replace("semver_check = true", "semver_check = false"), encoding="utf-8",
            )
            (repo / "CHANGELOG.md").write_text(
                "# Changelog\n\n## [Unreleased]\n\n## [0.4.0] - 2026-07-29\n",
                encoding="utf-8",
            )
            run("git", "init", "--quiet")
            run("git", "config", "user.name", "release-regression")
            run("git", "config", "user.email", "release-regression@example.invalid")
            run("git", "add", ".")
            run("git", "commit", "--quiet", "-m", "chore: release v0.4.0")
            run("git", "tag", "-a", "v0.4.0", "-m", "Previous shared release")
            workspace(repo, PACKAGES, "0.4.0")
            run("cargo", "generate-lockfile")
            run("git", "add", ".")
            run("git", "commit", "--quiet", "-m", "feat: add project-scoped agent")

            run(
                release_plz, "update", "--manifest-path", str(repo / "Cargo.toml"),
                "--registry-manifest-path", str(registry / "Cargo.toml"),
                "--release-date", "2026-11-04", "--repo-url", "https://github.com/sealtask/sealtask-oss",
            )
            manifest = tomllib.loads((repo / "Cargo.toml").read_text(encoding="utf-8"))
            self.assertEqual(manifest["workspace"]["package"]["version"], "0.5.0")
            for name in PACKAGES[1:]:
                package = tomllib.loads((repo / name / "Cargo.toml").read_text(encoding="utf-8"))
                self.assertEqual(package["dependencies"]["sealtask-client-core"]["version"], "=0.5.0")
            changelog = (repo / "CHANGELOG.md").read_text(encoding="utf-8")
            self.assertIn("0.5.0", changelog)
            self.assertIn("add project-scoped agent", changelog)
            self.assertEqual(run("git", "tag", "--list").strip(), "v0.4.0")

            # Once all seven packages have shipped, a normal fix must still
            # advance the same group and update every exact dependency pin.
            workspace(registry, PACKAGES, "0.5.0")
            run("git", "add", ".")
            run("git", "commit", "--quiet", "-m", "chore: release v0.5.0")
            run("git", "tag", "-a", "v0.5.0", "-m", "First agent release")
            run(
                release_plz, "update", "--manifest-path", str(repo / "Cargo.toml"),
                "--registry-manifest-path", str(registry / "Cargo.toml"),
                "--release-date", "2026-11-05", "--repo-url", "https://github.com/sealtask/sealtask-oss",
            )
            self.assertEqual(run("git", "status", "--porcelain"), "")
            (repo / "sealtask-client-core/src/lib.rs").write_text(
                "pub fn fixture() { let _fixed = true; }\n", encoding="utf-8",
            )
            run("git", "add", ".")
            run("git", "commit", "--quiet", "-m", "fix: repair core behavior")
            run(
                release_plz, "update", "--manifest-path", str(repo / "Cargo.toml"),
                "--registry-manifest-path", str(registry / "Cargo.toml"),
                "--release-date", "2026-11-05", "--repo-url", "https://github.com/sealtask/sealtask-oss",
            )
            manifest = tomllib.loads((repo / "Cargo.toml").read_text(encoding="utf-8"))
            self.assertEqual(manifest["workspace"]["package"]["version"], "0.5.1")
            for name in PACKAGES[1:]:
                package = tomllib.loads((repo / name / "Cargo.toml").read_text(encoding="utf-8"))
                self.assertEqual(package["dependencies"]["sealtask-client-core"]["version"], "=0.5.1")


if __name__ == "__main__":
    unittest.main()
