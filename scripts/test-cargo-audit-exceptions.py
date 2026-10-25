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

    def reject_lock(self, packages, message):
        with self.assertRaisesRegex(ValueError, message):
            GUARD.check_lock(packages, "oss", self.policy)

    def test_reviewed_workspace_passes(self):
        GUARD.check(self.workspace, "oss", REVIEW_DATE)

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

    def test_additional_ignore_fails(self):
        path = self.workspace / ".cargo/audit.toml"
        path.write_text(path.read_text().replace("ignore = [", 'ignore = ["RUSTSEC-2026-0000",'))
        with self.assertRaisesRegex(ValueError, "unreviewed audit ignore list"):
            GUARD.check(self.workspace, "oss", REVIEW_DATE)


if __name__ == "__main__":
    unittest.main()
