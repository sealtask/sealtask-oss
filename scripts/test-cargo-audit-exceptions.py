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

    def root_packages(self):
        """The reviewed crypto closure plus the root lockfile's SQLite route."""
        checksums = self.policy["sqlite_packages"]

        def registry(name, version, *dependencies):
            return {"name": name, "version": version, "source": GUARD.REGISTRY,
                    "checksum": checksums.get(f"{name} {version}", "f" * 64),
                    "dependencies": list(dependencies)}

        # The DOCX exceptions are OSS-only, so the root closure must not carry them.
        docx = {key.split()[0] for key in self.policy["docx_packages"]}
        packages = []
        for package in copy.deepcopy(self.packages):
            if package["name"] in docx:
                continue
            package["dependencies"] = [dependency for dependency in package.get("dependencies", [])
                                       if dependency.split()[0] not in docx]
            packages.append(package)
        return packages + [
            registry("libsqlite3-sys", "0.37.0"),
            registry("sqlx-sqlite", "0.9.0", "libsqlite3-sys 0.37.0"),
            registry("sqlx-macros-core", "0.9.0", "sqlx-sqlite 0.9.0"),
            registry("sqlx-macros", "0.9.0", "sqlx-macros-core 0.9.0"),
            registry("sqlx", "0.9.0", "sqlx-macros 0.9.0", "sqlx-sqlite 0.9.0"),
            {"name": "worklist-persistence", "version": "0.1.0", "dependencies": ["sqlx 0.9.0"]},
        ]

    def reject_root_lock(self, packages, message):
        with self.assertRaisesRegex(ValueError, message):
            GUARD.check_lock(packages, "root", self.policy)

    def test_reviewed_root_sqlite_route_passes(self):
        GUARD.check_lock(self.root_packages(), "root", self.policy)

    def test_new_libsqlite_consumer_fails(self):
        packages = self.root_packages()
        packages.append({"name": "rusqlite", "version": "0.38.0", "source": GUARD.REGISTRY,
                         "dependencies": ["libsqlite3-sys 0.37.0"]})
        self.reject_root_lock(packages, "unreviewed dependency path")

    def test_registry_sqlx_consumer_fails(self):
        packages = self.root_packages()
        packages.append({"name": "sqlx-adapter", "version": "1.0.0", "source": GUARD.REGISTRY,
                         "dependencies": ["sqlx 0.9.0"]})
        self.reject_root_lock(packages, "unreviewed dependency path to sqlx")

    def test_additional_libsqlite_version_fails(self):
        packages = self.root_packages()
        packages.append({"name": "libsqlite3-sys", "version": "0.38.0"})
        self.reject_root_lock(packages, "unreviewed affected version")

    def test_upgraded_libsqlite_requires_exception_removal(self):
        packages = self.root_packages()
        next(p for p in packages if p["name"] == "libsqlite3-sys")["version"] = "0.39.0"
        next(p for p in packages if p["name"] == "sqlx-sqlite")["dependencies"] = ["libsqlite3-sys 0.39.0"]
        self.reject_root_lock(packages, "changed; reassess/remove its exception")

    def test_uninspected_local_sqlx_consumer_fails(self):
        packages = [
            {"name": "sqlx", "version": "0.9.0", "source": GUARD.REGISTRY},
            {"name": "worklist-persistence", "version": "0.1.0", "dependencies": ["sqlx"]},
            {"name": "implicit-member", "version": "0.1.0", "dependencies": ["sqlx 0.9.0"]},
        ]
        manifests = [{"workspace": {"members": ["crates/persistence"]}},
                     {"package": {"name": "worklist-persistence"}}]
        GUARD.check_sqlx_consumers_inspected(packages[:2], manifests)
        with self.assertRaisesRegex(ValueError, "sqlx consumers outside the inspected workspace members"):
            GUARD.check_sqlx_consumers_inspected(packages, manifests)

    def test_root_sources_reject_sqlx_consumers_missing_from_members(self):
        root = Path(self.temp.name) / "root"
        suite = "impl CipherSuite for DefaultCipherSuite {\n    type OprfCs = Ristretto255;\n" \
                "    type KeyExchange = TripleDh<Ristretto255, Sha512>;\n    type Ksf = Argon2<'static>;\n}\n"
        files = {
            "Cargo.toml": '[workspace]\nmembers = ["crates/crypto"]\n',
            "crates/crypto/Cargo.toml": '[package]\nname = "worklist-crypto"\nversion = "0.1.0"\n',
            "crates/crypto/src/opaque.rs": suite,
        }
        for relative, text in files.items():
            (root / relative).parent.mkdir(parents=True, exist_ok=True)
            (root / relative).write_text(text)
        member = {"name": "worklist-crypto", "version": "0.1.0", "dependencies": ["sqlx"]}
        implicit = {"name": "implicit-member", "version": "0.1.0", "dependencies": ["sqlx"]}

        GUARD.check_sources(root, "root", self.policy, [member])
        with self.assertRaisesRegex(ValueError, r"inspected workspace members \['implicit-member'\]"):
            GUARD.check_sources(root, "root", self.policy, [member, implicit])

    def test_postgres_only_sqlx_features_pass(self):
        GUARD.check_sqlx_features([
            {"workspace": {"dependencies": {"sqlx": {"version": "0.9.0", "features": ["postgres"]}}}},
            {"dependencies": {"sqlx": {"workspace": True}},
             "features": {"sqlx_postgres": ["sqlx", "sqlx/postgres"]}},
        ])
        GUARD.check_sqlx_features([
            {"workspace": {"dependencies": {"db": {"package": "sqlx", "features": ["postgres"]}}}},
            {"dependencies": {"db": {"workspace": True, "features": ["migrate"]}},
             "features": {"pg": ["db/postgres"]}},
        ])

    def test_sqlx_sqlite_features_fail(self):
        renamed_workspace = {"workspace": {"dependencies": {
            "db": {"package": "sqlx", "version": "0.9.0", "features": ["postgres"]}}}}
        cases = {
            "workspace dependency": [{"workspace": {"dependencies": {
                "sqlx": {"version": "0.9.0", "features": ["sqlite"]}}}}],
            "member feature": [{"features": {"local-db": ["sqlx?/sqlite-unbundled"]}}],
            "renamed dependency": [{"dev-dependencies": {
                "db": {"package": "sqlx", "features": ["all-databases"]}}}],
            "target dependency": [{"target": {"cfg(unix)": {"dependencies": {
                "sqlx": {"version": "0.9.0", "features": ["_unstable-docs"]}}}}}],
            "inherited renamed dependency": [renamed_workspace, {"dependencies": {
                "db": {"workspace": True, "features": ["sqlite"]}}}],
            "inherited renamed feature forwarding": [renamed_workspace, {
                "dependencies": {"db": {"workspace": True}},
                "features": {"local": ["db/sqlite"]}}],
        }
        for label, manifests in cases.items():
            with self.subTest(label), self.assertRaisesRegex(ValueError, "SQLite support enabled"):
                GUARD.check_sqlx_features(manifests)

    def test_additional_ignore_fails(self):
        path = self.workspace / ".cargo/audit.toml"
        path.write_text(path.read_text().replace("ignore = [", 'ignore = ["RUSTSEC-2026-0000",'))
        with self.assertRaisesRegex(ValueError, "unreviewed audit ignore list"):
            GUARD.check(self.workspace, "oss", REVIEW_DATE)


if __name__ == "__main__":
    unittest.main()
