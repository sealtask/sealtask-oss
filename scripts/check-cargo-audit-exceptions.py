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
ZIP = {"RUSTSEC-2026-0367", "RUSTSEC-2026-0368", "RUSTSEC-2026-0369", "RUSTSEC-2026-0429"}
SQLITE = "RUSTSEC-2026-0459"
SQLITE_CRATES = {"libsqlite3-sys", "rusqlite", "sqlx-sqlite"}
SQLITE_ROUTES = {
    ("libsqlite3-sys", "0.37.0"): {("sqlx-sqlite", "0.9.0")},
    ("sqlx-sqlite", "0.9.0"): {("sqlx", "0.9.0"), ("sqlx-macros-core", "0.9.0")},
    ("sqlx-macros-core", "0.9.0"): {("sqlx-macros", "0.9.0")},
    ("sqlx-macros", "0.9.0"): {("sqlx", "0.9.0")},
}
PATCHED = {"crypto-bigint": (0, 7, 6), "zip": (9, 0, 0), "libsqlite3-sys": (0, 39, 0)}


def require(condition, message):
    if not condition:
        raise ValueError(message)


def index_lock(packages):
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
    return by_key, parents


def check_sqlite_routes(by_key, parents):
    for package, allowed in SQLITE_ROUTES.items():
        require(parents.get(package) == allowed, f"unreviewed dependency path to {package}")
    sqlx_parents = parents.get(("sqlx", "0.9.0"), set())
    require(
        sqlx_parents and all("source" not in by_key[parent] for parent in sqlx_parents),
        "sqlx must be consumed only by local workspace crates",
    )


def check_lock(packages, scope, policy, workspace_version=None):
    by_key, parents = index_lock(packages)

    reviewed = dict(policy["crypto_packages"])
    if scope == "oss":
        reviewed.update(policy["docx_packages"])
    if scope == "root":
        reviewed.update(policy["sqlite_packages"])
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
        runtime = ("sealtask-client-runtime", workspace_version)
        require(runtime in by_key, "DOCX consumer must match the workspace version")
        require("source" not in by_key[runtime], "DOCX consumer must be the local runtime crate")
        expected_parents.update({
            ("zip", "2.6.1"): {("undocx", "0.5.2")},
            ("zip", "4.6.1"): {("rs-docx", "0.2.0")},
            ("rs-docx", "0.2.0"): {("undocx", "0.5.2")},
            ("undocx", "0.5.2"): {runtime},
        })
    for package, allowed in expected_parents.items():
        require(parents[package] == allowed, f"unreviewed dependency path to {package}")
    if scope == "root":
        check_sqlite_routes(by_key, parents)
    for name, version in by_key:
        if name not in PATCHED:
            continue
        parts = version.split(".")
        require(all(p.isdigit() for p in parts), f"unreviewed prerelease: {name} {version}")
        if tuple(map(int, parts)) < PATCHED[name]:
            require(f"{name} {version}" in reviewed, f"unreviewed affected version: {name} {version}")


def dependency_tables(manifest):
    for section in ("dependencies", "dev-dependencies", "build-dependencies"):
        yield manifest.get(section, {})
    for target in manifest.get("target", {}).values():
        for section in ("dependencies", "dev-dependencies", "build-dependencies"):
            yield target.get(section, {})


def check_sqlite_disabled(workspace, members):
    manifests = {"Cargo.toml": tomllib.loads((workspace / "Cargo.toml").read_text())}
    for member in members:
        path = workspace / member / "Cargo.toml"
        if path.exists():
            manifests[f"{member}/Cargo.toml"] = tomllib.loads(path.read_text())
    for relative, manifest in manifests.items():
        tables = list(dependency_tables(manifest))
        tables.append(manifest.get("workspace", {}).get("dependencies", {}))
        for table in tables:
            for name, spec in table.items():
                package = spec.get("package", name) if isinstance(spec, dict) else name
                require(package not in SQLITE_CRATES, f"{relative} depends on SQLite crate {package}")
                features = spec.get("features", []) if isinstance(spec, dict) else []
                if package.startswith("sqlx"):
                    require(
                        not any("sqlite" in feature for feature in features),
                        f"{relative} enables SQLite through {package}",
                    )
        for feature, enables in manifest.get("features", {}).items():
            for item in enables:
                dependency, _, enabled = item.partition("/")
                require(
                    not (dependency.rstrip("?").startswith("sqlx") and "sqlite" in enabled),
                    f"{relative} feature {feature} enables SQLite through {item}",
                )


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
    if scope == "root":
        check_sqlite_disabled(workspace, members)


def check(workspace, scope, today=None):
    policy = json.loads(POLICY_PATH.read_text())
    today = today or datetime.datetime.now(datetime.timezone.utc).date()
    require(today <= datetime.date.fromisoformat(policy["expires"]), "audit exceptions expired")
    settings = policy["scopes"][scope]
    config = tomllib.loads((workspace / settings["config"]).read_text())
    ignored = config.get("advisories", {}).get("ignore", [])
    expected = {CRYPTO} | (ZIP if scope == "oss" else {SQLITE})
    require(set(ignored) == expected and len(ignored) == len(expected), "unreviewed audit ignore list")
    packages = tomllib.loads((workspace / "Cargo.lock").read_text())["package"]
    manifest = tomllib.loads((workspace / "Cargo.toml").read_text())
    workspace_version = manifest.get("workspace", {}).get("package", {}).get("version")
    check_lock(packages, scope, policy, workspace_version)
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
