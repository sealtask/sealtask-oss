#!/usr/bin/env python3
"""Validate the local metadata that makes an OSS release reproducible."""

from __future__ import annotations

import argparse
import datetime as dt
import json
import os
import re
import subprocess
import sys
from dataclasses import dataclass
from pathlib import Path
from typing import Any, Iterable, Sequence


PUBLISH_ORDER = (
    "sealtask-client-core",
    "sealtask-client-auth",
    "sealtask-client-crypto",
    "sealtask-client-api",
    "sealtask-client-runtime",
    "sealtask",
)

EXPECTED_MANIFESTS = {
    "sealtask-client-core": Path("crates/client-core/Cargo.toml"),
    "sealtask-client-auth": Path("crates/client-auth/Cargo.toml"),
    "sealtask-client-crypto": Path("crates/client-crypto/Cargo.toml"),
    "sealtask-client-api": Path("crates/client-api/Cargo.toml"),
    "sealtask-client-runtime": Path("crates/client-runtime/Cargo.toml"),
    "sealtask": Path("cli/Cargo.toml"),
}

# This is the strict SemVer 2.0 grammar from semver.org. In particular, it
# rejects leading zeroes in numeric identifiers.
SEMVER_PATTERN = re.compile(
    r"^(0|[1-9]\d*)\."
    r"(0|[1-9]\d*)\."
    r"(0|[1-9]\d*)"
    r"(?:-("
    r"(?:0|[1-9]\d*|\d*[A-Za-z-][0-9A-Za-z-]*)"
    r"(?:\.(?:0|[1-9]\d*|\d*[A-Za-z-][0-9A-Za-z-]*))*"
    r"))?"
    r"(?:\+([0-9A-Za-z-]+(?:\.[0-9A-Za-z-]+)*))?$"
)

MAN_HEADER_PATTERN = re.compile(
    r'^\.TH\s+\S+\s+1\s+.*?"SealTask CLI ([^"]+)"\s+"SealTask Manual"\s*$',
    re.MULTILINE,
)


class ValidationFailure(Exception):
    """A deterministic collection of release metadata validation errors."""

    def __init__(self, messages: Iterable[str]):
        unique_messages = tuple(dict.fromkeys(messages))
        if not unique_messages:
            raise ValueError("ValidationFailure requires at least one message")
        self.messages = unique_messages
        super().__init__("\n".join(unique_messages))


@dataclass(frozen=True)
class ValidationReport:
    version: str
    man_page_count: int


def _display_list(values: Iterable[str]) -> str:
    return "[" + ", ".join(values) + "]"


def _display_path(path_value: object, workspace_root: Path) -> str:
    if not isinstance(path_value, str) or not path_value:
        return "<none>"

    path = Path(path_value).resolve()
    try:
        return path.relative_to(workspace_root).as_posix()
    except ValueError:
        return path.as_posix()


def _is_publishable(package: dict[str, Any]) -> bool:
    publish = package.get("publish")
    return publish is None or (isinstance(publish, list) and bool(publish))


def _workspace_packages(metadata: dict[str, Any], errors: list[str]) -> list[dict[str, Any]]:
    raw_packages = metadata.get("packages")
    raw_members = metadata.get("workspace_members")
    if not isinstance(raw_packages, list):
        errors.append("cargo metadata did not contain a packages array")
        return []
    if not isinstance(raw_members, list):
        errors.append("cargo metadata did not contain a workspace_members array")
        return []

    packages_by_id: dict[str, dict[str, Any]] = {}
    for package in raw_packages:
        if not isinstance(package, dict) or not isinstance(package.get("id"), str):
            errors.append("cargo metadata contained a package without a valid id")
            continue
        packages_by_id[package["id"]] = package

    members: list[dict[str, Any]] = []
    for member_id in raw_members:
        package = packages_by_id.get(member_id)
        if package is None:
            errors.append(f"workspace member {member_id!r} was missing from cargo metadata packages")
            continue
        members.append(package)
    return members


def _packages_by_name(
    packages: Sequence[dict[str, Any]], errors: list[str]
) -> dict[str, dict[str, Any]]:
    result: dict[str, dict[str, Any]] = {}
    for package in packages:
        name = package.get("name")
        if not isinstance(name, str) or not name:
            errors.append("cargo metadata contained a workspace package without a valid name")
            continue
        if name in result:
            errors.append(f"cargo metadata contained duplicate workspace package name {name!r}")
            continue
        result[name] = package
    return result


def _validate_publishable_packages(
    packages: Sequence[dict[str, Any]],
    packages_by_name: dict[str, dict[str, Any]],
    workspace_root: Path,
    errors: list[str],
) -> None:
    actual_publishable = sorted(
        package["name"]
        for package in packages
        if isinstance(package.get("name"), str) and _is_publishable(package)
    )
    expected_publishable = sorted(PUBLISH_ORDER)
    if actual_publishable != expected_publishable:
        errors.append(
            "publishable workspace crates must be exactly "
            f"{_display_list(expected_publishable)}; "
            f"found {_display_list(actual_publishable)}"
        )

    for name in PUBLISH_ORDER:
        package = packages_by_name.get(name)
        if package is None:
            errors.append(f"required publishable crate {name!r} is missing from the workspace")
            continue

        publish = package.get("publish")
        if publish != ["crates-io"]:
            errors.append(
                f"{name} must set publish = [\"crates-io\"]; "
                f"cargo metadata reported {json.dumps(publish, sort_keys=True)}"
            )

        expected_manifest = (workspace_root / EXPECTED_MANIFESTS[name]).resolve()
        actual_manifest_value = package.get("manifest_path")
        actual_manifest = (
            Path(actual_manifest_value).resolve()
            if isinstance(actual_manifest_value, str)
            else None
        )
        if actual_manifest != expected_manifest:
            errors.append(
                f"{name} manifest must be {EXPECTED_MANIFESTS[name].as_posix()}; "
                f"found {_display_path(actual_manifest_value, workspace_root)}"
            )


def _shared_version(
    packages_by_name: dict[str, dict[str, Any]], errors: list[str]
) -> str | None:
    if any(name not in packages_by_name for name in PUBLISH_ORDER):
        return None

    versions_by_name = {
        name: packages_by_name[name].get("version") for name in PUBLISH_ORDER
    }
    invalid_version_values = [
        name
        for name, version in versions_by_name.items()
        if not isinstance(version, str) or not version
    ]
    for name in invalid_version_values:
        errors.append(f"{name} has no valid version in cargo metadata")
    if invalid_version_values:
        return None

    versions = {str(version) for version in versions_by_name.values()}
    if len(versions) != 1:
        details = ", ".join(
            f"{name}={versions_by_name[name]}" for name in PUBLISH_ORDER
        )
        errors.append(f"publishable crates must share one workspace version; found {details}")
        return None

    version = next(iter(versions))
    if SEMVER_PATTERN.fullmatch(version) is None:
        errors.append(f"workspace version {version!r} is not valid SemVer 2.0")
        return None
    return version


def _internal_dependencies(
    package: dict[str, Any],
) -> list[dict[str, Any]]:
    dependencies = package.get("dependencies")
    if not isinstance(dependencies, list):
        return []
    return [
        dependency
        for dependency in dependencies
        if isinstance(dependency, dict) and dependency.get("name") in EXPECTED_MANIFESTS
    ]


def _validate_internal_dependencies(
    packages_by_name: dict[str, dict[str, Any]],
    workspace_root: Path,
    version: str,
    errors: list[str],
) -> None:
    order_index = {name: index for index, name in enumerate(PUBLISH_ORDER)}
    exact_requirement = f"={version}"

    for package_name in PUBLISH_ORDER:
        package = packages_by_name.get(package_name)
        if package is None:
            continue

        dependencies = sorted(
            _internal_dependencies(package),
            key=lambda dependency: (
                str(dependency.get("name")),
                str(dependency.get("kind")),
                str(dependency.get("target")),
            ),
        )
        for dependency in dependencies:
            dependency_name = str(dependency["name"])
            requirement = dependency.get("req")
            if requirement != exact_requirement:
                errors.append(
                    f"{package_name} dependency {dependency_name} must require "
                    f"{exact_requirement!r}; found {requirement!r}"
                )

            expected_path = (
                workspace_root / EXPECTED_MANIFESTS[dependency_name].parent
            ).resolve()
            actual_path_value = dependency.get("path")
            actual_path = (
                Path(actual_path_value).resolve()
                if isinstance(actual_path_value, str)
                else None
            )
            if actual_path != expected_path:
                errors.append(
                    f"{package_name} dependency {dependency_name} must use path "
                    f"{EXPECTED_MANIFESTS[dependency_name].parent.as_posix()!r}; "
                    f"found {_display_path(actual_path_value, workspace_root)!r}"
                )

            if order_index[dependency_name] >= order_index[package_name]:
                errors.append(
                    "publication order is not topological: "
                    f"{package_name} depends on later crate {dependency_name}"
                )


def _validate_man_pages(
    workspace_root: Path, version: str, errors: list[str]
) -> int:
    man_directory = workspace_root / "cli/assets/man"
    man_pages = sorted(man_directory.glob("*.1")) if man_directory.is_dir() else []
    if not man_pages:
        errors.append("cli/assets/man must contain at least one generated .1 man page")
        return 0

    for man_page in man_pages:
        relative_path = man_page.relative_to(workspace_root).as_posix()
        try:
            contents = man_page.read_text(encoding="utf-8")
        except OSError as error:
            errors.append(f"could not read {relative_path}: {error}")
            continue

        header_versions = MAN_HEADER_PATTERN.findall(contents)
        if len(header_versions) != 1:
            errors.append(
                f"{relative_path} must contain exactly one SealTask CLI man-page header; "
                f"found {len(header_versions)}"
            )
            continue
        if header_versions[0] != version:
            errors.append(
                f"{relative_path} has CLI version {header_versions[0]!r}; "
                f"expected {version!r}"
            )
    return len(man_pages)


def _validate_release_inputs(
    workspace_root: Path, version: str, tag: str | None, errors: list[str]
) -> None:
    expected_tag = f"v{version}"
    if tag != expected_tag:
        errors.append(f"release tag must be {expected_tag!r}; found {tag!r}")

    changelog_path = workspace_root / "CHANGELOG.md"
    try:
        changelog_lines = changelog_path.read_text(encoding="utf-8").splitlines()
    except FileNotFoundError:
        errors.append("release mode requires CHANGELOG.md")
        return
    except OSError as error:
        errors.append(f"could not read CHANGELOG.md: {error}")
        return

    version_heading_pattern = re.compile(
        rf"^## \[{re.escape(version)}\](?:\s.*)?$"
    )
    version_headings = [
        line for line in changelog_lines if version_heading_pattern.fullmatch(line)
    ]
    if len(version_headings) != 1:
        errors.append(
            "CHANGELOG.md must contain exactly one heading for "
            f"{version!r}; found {len(version_headings)}"
        )
        return

    exact_heading_pattern = re.compile(
        rf"^## \[{re.escape(version)}\] - (\d{{4}}-\d{{2}}-\d{{2}})$"
    )
    exact_heading = exact_heading_pattern.fullmatch(version_headings[0])
    if exact_heading is None:
        errors.append(
            "CHANGELOG.md release heading must match "
            f"'## [{version}] - YYYY-MM-DD'; found {version_headings[0]!r}"
        )
        return

    try:
        dt.date.fromisoformat(exact_heading.group(1))
    except ValueError:
        errors.append(
            f"CHANGELOG.md release heading has invalid date {exact_heading.group(1)!r}"
        )


def validate_release_metadata(
    metadata: dict[str, Any],
    manifest_path: Path,
    mode: str,
    tag: str | None = None,
) -> ValidationReport:
    """Validate parsed Cargo metadata and adjacent checked-in release assets."""

    errors: list[str] = []
    workspace_root_value = metadata.get("workspace_root")
    if not isinstance(workspace_root_value, str) or not workspace_root_value:
        raise ValidationFailure(["cargo metadata did not contain a valid workspace_root"])

    workspace_root = Path(workspace_root_value).resolve()
    expected_workspace_root = manifest_path.resolve().parent
    if workspace_root != expected_workspace_root:
        errors.append(
            "cargo metadata workspace_root does not match the manifest directory: "
            f"expected {expected_workspace_root.as_posix()!r}, "
            f"found {workspace_root.as_posix()!r}"
        )

    packages = _workspace_packages(metadata, errors)
    packages_by_name = _packages_by_name(packages, errors)
    _validate_publishable_packages(packages, packages_by_name, workspace_root, errors)
    version = _shared_version(packages_by_name, errors)

    man_page_count = 0
    if version is not None:
        _validate_internal_dependencies(
            packages_by_name, workspace_root, version, errors
        )
        man_page_count = _validate_man_pages(workspace_root, version, errors)
        if mode == "release":
            _validate_release_inputs(workspace_root, version, tag, errors)

    if errors:
        raise ValidationFailure(errors)
    if version is None:
        raise ValidationFailure(["could not determine the shared workspace version"])
    return ValidationReport(version=version, man_page_count=man_page_count)


def load_cargo_metadata(manifest_path: Path, cargo: str) -> dict[str, Any]:
    """Load workspace packages without resolving dependencies or using the network."""

    command = [
        cargo,
        "metadata",
        "--manifest-path",
        str(manifest_path),
        "--format-version",
        "1",
        "--locked",
        "--no-deps",
        "--offline",
    ]
    environment = os.environ.copy()
    environment["CARGO_NET_OFFLINE"] = "true"
    try:
        result = subprocess.run(
            command,
            check=False,
            capture_output=True,
            text=True,
            env=environment,
        )
    except OSError as error:
        raise ValidationFailure([f"could not run cargo metadata: {error}"]) from error

    if result.returncode != 0:
        detail = result.stderr.strip() or result.stdout.strip() or "no diagnostic output"
        raise ValidationFailure(
            [f"cargo metadata failed with exit code {result.returncode}: {detail}"]
        )
    try:
        metadata = json.loads(result.stdout)
    except json.JSONDecodeError as error:
        raise ValidationFailure(
            [f"cargo metadata returned invalid JSON: {error.msg}"]
        ) from error
    if not isinstance(metadata, dict):
        raise ValidationFailure(["cargo metadata JSON root must be an object"])
    return metadata


def _argument_parser() -> argparse.ArgumentParser:
    default_manifest = Path(__file__).resolve().parent.parent / "Cargo.toml"
    parser = argparse.ArgumentParser(
        description="Validate the checked-in metadata for an OSS CLI release."
    )
    parser.add_argument("mode", choices=("workspace", "release"))
    parser.add_argument(
        "--tag",
        help="immutable release tag (required in release mode, for example v1.2.3)",
    )
    parser.add_argument(
        "--manifest-path",
        type=Path,
        default=default_manifest,
        help=argparse.SUPPRESS,
    )
    parser.add_argument(
        "--cargo",
        default=os.environ.get("CARGO", "cargo"),
        help=argparse.SUPPRESS,
    )
    return parser


def main(argv: Sequence[str] | None = None) -> int:
    parser = _argument_parser()
    arguments = parser.parse_args(argv)
    if arguments.mode == "release" and arguments.tag is None:
        parser.error("release mode requires --tag")
    if arguments.mode == "workspace" and arguments.tag is not None:
        parser.error("workspace mode does not accept --tag")

    manifest_path = arguments.manifest_path.resolve()
    if not manifest_path.is_file():
        print(f"error: manifest does not exist: {manifest_path}", file=sys.stderr)
        return 1

    try:
        metadata = load_cargo_metadata(manifest_path, arguments.cargo)
        report = validate_release_metadata(
            metadata, manifest_path, arguments.mode, arguments.tag
        )
    except ValidationFailure as error:
        for message in error.messages:
            print(f"error: {message}", file=sys.stderr)
        return 1

    tag_summary = f" tag={arguments.tag}" if arguments.tag is not None else ""
    print(
        "release metadata valid: "
        f"mode={arguments.mode} version={report.version}{tag_summary} "
        f"publishable_crates={len(PUBLISH_ORDER)} "
        f"man_pages={report.man_page_count}"
    )
    print(f"publication order: {' -> '.join(PUBLISH_ORDER)}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
