#!/usr/bin/env python3
"""Exercise the rejection paths of the temporary audit exception guard."""

import copy
import datetime
import importlib.util
import json
from pathlib import Path
import shutil
import tempfile
import tomllib
import unittest

SCRIPT = Path(__file__).with_name("check-cargo-audit-exceptions.py")
SPEC = importlib.util.spec_from_file_location("audit_guard", SCRIPT)
GUARD = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(GUARD)
OSS = SCRIPT.parent.parent
REVIEW_DATE = datetime.date(2026, 10, 25)


class AuditExceptionTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.workspace = Path(self.temp.name)
        self.policy = json.loads(GUARD.POLICY_PATH.read_text())
        paths = ["Cargo.toml", "Cargo.lock", ".cargo/audit.toml",
                 self.policy["docx_reader"]["path"]]
        paths += [key.split(":")[0] for key in self.policy["scopes"]["oss"]["suites"]]
        for relative in paths:
            destination = self.workspace / relative
            destination.parent.mkdir(parents=True, exist_ok=True)
            shutil.copyfile(OSS / relative, destination)
        self.packages = tomllib.loads((OSS / "Cargo.lock").read_text())["package"]
        self.version = tomllib.loads((OSS / "Cargo.toml").read_text())["workspace"]["package"]["version"]
        major, minor, _ = self.version.split(".", 2)
        self.next_version = f"{major}.{int(minor) + 1}.0"

    def reject_lock(self, packages, message):
        with self.assertRaisesRegex(ValueError, message):
            GUARD.check_lock(packages, "oss", self.policy, self.version)

    def test_reviewed_workspace_passes(self):
        GUARD.check(self.workspace, "oss", REVIEW_DATE)

    def test_release_version_bump_preserves_reviewed_dependency_path(self):
        runtime = next(p for p in self.packages if p["name"] == "sealtask-client-runtime")
        runtime["version"] = self.next_version
        GUARD.check_lock(self.packages, "oss", self.policy, self.next_version)

    def test_mismatched_runtime_version_fails(self):
        with self.assertRaisesRegex(ValueError, "must match the workspace version"):
            GUARD.check_lock(self.packages, "oss", self.policy, self.next_version)

    def test_registry_runtime_substitution_fails(self):
        runtime = next(p for p in self.packages if p["name"] == "sealtask-client-runtime")
        runtime["source"] = GUARD.REGISTRY
        self.reject_lock(self.packages, "must be the local runtime crate")

    def test_expiry_fails(self):
        with self.assertRaisesRegex(ValueError, "expired"):
            GUARD.check(self.workspace, "oss", datetime.date(2026, 11, 26))

    def test_new_bigint_consumer_fails(self):
        self.packages.append({"name": "new-consumer", "version": "1.0.0",
                              "dependencies": ["crypto-bigint 0.5.5"]})
        self.reject_lock(self.packages, "unreviewed dependency path")

    def test_new_elliptic_curve_consumer_fails(self):
        self.packages.append({"name": "p256", "version": "0.13.2",
                              "dependencies": ["elliptic-curve 0.13.8"]})
        self.reject_lock(self.packages, "unreviewed dependency path")

    def test_new_zip_consumer_fails(self):
        self.packages.append({"name": "extractor", "version": "1.0.0",
                              "dependencies": ["zip 2.6.1"]})
        self.reject_lock(self.packages, "unreviewed dependency path")

    def test_substituted_registry_source_fails(self):
        packages = copy.deepcopy(self.packages)
        next(p for p in packages if p["name"] == "undocx")["checksum"] = "0" * 64
        self.reject_lock(packages, "reviewed registry source")

    def test_additional_affected_version_fails(self):
        self.packages.append({"name": "zip", "version": "8.0.0"})
        self.reject_lock(self.packages, "unreviewed affected version")

    def test_non_ristretto_suite_fails(self):
        path = self.workspace / "crates/client-auth/src/lib.rs"
        path.write_text(path.read_text().replace("type OprfCs = Ristretto255;", "type OprfCs = p256::NistP256;"))
        with self.assertRaisesRegex(ValueError, "cipher suite declarations changed"):
            GUARD.check(self.workspace, "oss", REVIEW_DATE)

    def test_changed_docx_reader_fails(self):
        path = self.workspace / self.policy["docx_reader"]["path"]
        source = path.read_text()
        self.assertIn(".skip_images()", source)
        path.write_text(source.replace(".skip_images()", ""))
        with self.assertRaisesRegex(ValueError, "DOCX reader changed"):
            GUARD.check(self.workspace, "oss", REVIEW_DATE)

    def test_new_docx_call_site_fails(self):
        path = self.workspace / "crates/client-auth/src/unreviewed.rs"
        path.write_text("use undocx::DocxToMd;\n")
        with self.assertRaisesRegex(ValueError, "DOCX conversion call site"):
            GUARD.check(self.workspace, "oss", REVIEW_DATE)

    def test_unreviewed_sqlite_version_fails_outside_root(self):
        self.packages.append({"name": "libsqlite3-sys", "version": "0.37.0"})
        self.reject_lock(self.packages, "unreviewed affected version")

    def test_additional_ignore_fails(self):
        path = self.workspace / ".cargo/audit.toml"
        path.write_text(path.read_text().replace("ignore = [", 'ignore = ["RUSTSEC-2026-0000",'))
        with self.assertRaisesRegex(ValueError, "unreviewed audit ignore list"):
            GUARD.check(self.workspace, "oss", REVIEW_DATE)


class SqliteExceptionTests(unittest.TestCase):
    """Synthetic root-workspace fixtures, so the public OSS mirror can run them."""

    def setUp(self):
        self.policy = json.loads(GUARD.POLICY_PATH.read_text())
        checksums = self.policy["sqlite_packages"]

        def registry(name, version, dependencies=()):
            return {"name": name, "version": version, "source": GUARD.REGISTRY,
                    "checksum": checksums[f"{name} {version}"],
                    "dependencies": list(dependencies)}

        self.packages = [
            {"name": "worklist-persistence", "version": "0.1.0", "dependencies": ["sqlx"]},
            registry("sqlx", "0.9.0", ["sqlx-macros", "sqlx-sqlite"]),
            registry("sqlx-macros", "0.9.0", ["sqlx-macros-core"]),
            registry("sqlx-macros-core", "0.9.0", ["sqlx-sqlite"]),
            registry("sqlx-sqlite", "0.9.0", ["libsqlite3-sys"]),
            registry("libsqlite3-sys", "0.37.0"),
        ]
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.workspace = Path(self.temp.name)
        self.write("Cargo.toml", """
[workspace]
members = ["crates/db"]

[workspace.dependencies]
sqlx = { version = "0.9.0", default-features = false, features = ["postgres", "macros"] }
""")
        self.write("crates/db/Cargo.toml", """
[package]
name = "db"

[features]
postgres = ["sqlx/postgres"]

[dependencies]
sqlx = { workspace = true, features = ["migrate"] }
""")

    def write(self, relative, text):
        path = self.workspace / relative
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(text)

    def edit(self, relative, old, new):
        path = self.workspace / relative
        source = path.read_text()
        self.assertIn(old, source)
        path.write_text(source.replace(old, new))

    def check_routes(self):
        GUARD.check_sqlite_routes(*GUARD.index_lock(self.packages))

    def check_manifests(self):
        GUARD.check_sqlite_disabled(self.workspace, ["crates/db"])

    def test_reviewed_routes_and_manifests_pass(self):
        self.check_routes()
        self.check_manifests()

    def test_new_libsqlite_consumer_fails(self):
        self.packages.append({"name": "rusqlite", "version": "0.37.0", "source": GUARD.REGISTRY,
                              "dependencies": ["libsqlite3-sys"]})
        with self.assertRaisesRegex(ValueError, "unreviewed dependency path"):
            self.check_routes()

    def test_registry_sqlx_consumer_fails(self):
        self.packages.append({"name": "sqlx-adapter", "version": "1.0.0", "source": GUARD.REGISTRY,
                              "dependencies": ["sqlx"]})
        with self.assertRaisesRegex(ValueError, "only by local workspace crates"):
            self.check_routes()

    def test_workspace_sqlite_feature_fails(self):
        self.edit("Cargo.toml", '"macros"]', '"macros", "sqlite"]')
        with self.assertRaisesRegex(ValueError, "enables SQLite through sqlx"):
            self.check_manifests()

    def test_member_weak_sqlite_feature_fails(self):
        self.edit("crates/db/Cargo.toml", '["sqlx/postgres"]', '["sqlx/postgres", "sqlx?/sqlite-unbundled"]')
        with self.assertRaisesRegex(ValueError, "feature postgres enables SQLite"):
            self.check_manifests()

    def test_renamed_sqlite_dependency_fails(self):
        self.edit("crates/db/Cargo.toml", "[dependencies]\n",
                  '[dependencies]\nembedded = { version = "0.37", package = "libsqlite3-sys" }\n')
        with self.assertRaisesRegex(ValueError, "depends on SQLite crate libsqlite3-sys"):
            self.check_manifests()

    def test_target_specific_rusqlite_dependency_fails(self):
        manifest = self.workspace / "crates/db/Cargo.toml"
        manifest.write_text(manifest.read_text() + "\n[target.'cfg(unix)'.dev-dependencies]\nrusqlite = \"0.41\"\n")
        with self.assertRaisesRegex(ValueError, "depends on SQLite crate rusqlite"):
            self.check_manifests()


if __name__ == "__main__":
    unittest.main()
