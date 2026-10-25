#!/usr/bin/env python3
"""Reject audit exceptions outside the reviewed, temporary dependency scope."""

import argparse
import datetime
import hashlib
import json
from pathlib import Path
import re
import tomllib

REGISTRY = "registry+https://github.com/rust-lang/crates.io-index"
POLICY_PATH = Path(__file__).with_name("cargo-audit-exceptions.json")
CRYPTO = "RUSTSEC-2026-0387"
ZIP = {"RUSTSEC-2026-0367", "RUSTSEC-2026-0368", "RUSTSEC-2026-0369"}


def require(condition, message):
    if not condition:
        raise ValueError(message)


def check_lock(packages, scope, policy):
    by_key = {(p["name"], p["version"]): p for p in packages}
    parents = {key: set() for key in by_key}
    for parent, package in by_key.items():
        for dependency in package.get("dependencies", []):
            parts = dependency.split()
            matches = [
                key for key in by_key
                if key[0] == parts[0] and (len(parts) == 1 or key[1] == parts[1])
            ]
            require(len(matches) == 1, f"ambiguous lock dependency: {dependency}")
            parents[matches[0]].add(parent)

    reviewed = dict(policy["crypto_packages"])
    if scope == "oss":
        reviewed.update(policy["docx_packages"])
    for specifier, checksum in reviewed.items():
        key = tuple(specifier.split())
        package = by_key.get(key)
        require(package is not None, f"{specifier} changed; reassess/remove its exception")
        require(
            package.get("source") == REGISTRY and package.get("checksum") == checksum,
            f"{specifier} no longer matches the reviewed registry source",
        )

    expected_parents = {
        ("crypto-bigint", "0.5.5"): {("elliptic-curve", "0.13.8")},
        ("elliptic-curve", "0.13.8"): {("opaque-ke", "4.0.1"), ("voprf", "0.5.0"), ("ecdsa", "0.16.9")},
        ("ecdsa", "0.16.9"): {("opaque-ke", "4.0.1")},
        ("voprf", "0.5.0"): {("opaque-ke", "4.0.1")},
    }
    if scope == "oss":
        expected_parents.update({
            ("zip", "2.6.1"): {("undocx", "0.5.2")},
            ("zip", "4.6.1"): {("rs-docx", "0.2.0")},
            ("rs-docx", "0.2.0"): {("undocx", "0.5.2")},
            ("undocx", "0.5.2"): {("sealtask-client-runtime", "0.4.0")},
        })
    for package, allowed in expected_parents.items():
        require(parents[package] == allowed, f"unreviewed dependency path to {package}")
    for name, version in by_key:
        if name not in {"crypto-bigint", "zip"}:
            continue
        parts = version.split(".")
        require(all(p.isdigit() for p in parts), f"unreviewed prerelease: {name} {version}")
        patched = (0, 7, 6) if name == "crypto-bigint" else (9, 0, 0)
        if tuple(map(int, parts)) < patched:
            require(f"{name} {version}" in reviewed, f"unreviewed affected version: {name} {version}")


def check_sources(workspace, scope, policy):
    manifest = tomllib.loads((workspace / "Cargo.toml").read_text())
    members = manifest.get("workspace", {}).get("members", ["."])
    suites = {}
    docx_uses = set()
    for member in members:
        for directory in ("src", "tests", "examples", "benches"):
            for path in (workspace / member / directory).rglob("*.rs"):
                source = path.read_text()
                relative = path.relative_to(workspace).as_posix()
                if re.search(r"\bundocx\b", source):
                    docx_uses.add(relative)
                for name, body in re.findall(
                    r"impl\s+(?:opaque_ke::)?CipherSuite\s+for\s+(\w+)\s*\{([^{}]*)\}",
                    source,
                ):
                    key = f"{relative}:{name}"
                    require(key not in suites, f"duplicate cipher suite: {key}")
                    suites[key] = re.sub(r"\s+", "", body)
    require(
        suites == policy["scopes"][scope]["suites"],
        "OPAQUE cipher suite declarations changed; reassess the Ristretto-only exception",
    )
    if scope == "oss":
        reader = policy["docx_reader"]
        require(docx_uses == {reader["path"]}, "unreviewed DOCX conversion call site")
        require(
            hashlib.sha256((workspace / reader["path"]).read_bytes()).hexdigest() == reader["sha256"],
            "reviewed DOCX reader changed; recheck bounded in-memory, image-free conversion",
        )
    else:
        require(not docx_uses, "DOCX exceptions are confined to the OSS reader")


def check(workspace, scope, today=None):
    policy = json.loads(POLICY_PATH.read_text())
    today = today or datetime.datetime.now(datetime.timezone.utc).date()
    require(today <= datetime.date.fromisoformat(policy["expires"]), "audit exceptions expired")
    settings = policy["scopes"][scope]
    config = tomllib.loads((workspace / settings["config"]).read_text())
    ignored = config.get("advisories", {}).get("ignore", [])
    expected = {CRYPTO} | (ZIP if scope == "oss" else set())
    require(set(ignored) == expected and len(ignored) == len(expected), "unreviewed audit ignore list")
    packages = tomllib.loads((workspace / "Cargo.lock").read_text())["package"]
    check_lock(packages, scope, policy)
    check_sources(workspace, scope, policy)


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--workspace", type=Path, default=Path.cwd())
    parser.add_argument("--scope", choices=("root", "oss", "ios"), required=True)
    args = parser.parse_args()
    try:
        check(args.workspace.resolve(), args.scope)
    except (ValueError, KeyError, OSError) as error:
        parser.exit(1, f"cargo-audit exception scope: {error}\n")
    print(f"cargo-audit exception scope verified: {args.scope}")
