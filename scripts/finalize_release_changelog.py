#!/usr/bin/env python3
"""Normalize release-plz changelog output to the repository's link style."""

from __future__ import annotations

import argparse
import datetime as dt
from dataclasses import dataclass
from pathlib import Path
import re
import sys
import tomllib
from typing import Iterable


REPOSITORY_URL = "https://github.com/sealtask/sealtask-oss"
COMPARE_URL_PREFIX = f"{REPOSITORY_URL}/compare/"

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
CANONICAL_HEADING_PATTERN = re.compile(r"^## \[([^\]]+)\] - (\d{4}-\d{2}-\d{2})$")
INLINE_HEADING_PATTERN = re.compile(
    r"^## \[([^\]]+)\]\(([^)]+)\) - (\d{4}-\d{2}-\d{2})$"
)
LINK_DEFINITION_PATTERN = re.compile(r"^\[([^\]]+)\]: (.+)$")


@dataclass(frozen=True)
class FinalizedRelease:
    version: str
    previous_version: str
    release_date: dt.date


@dataclass(frozen=True)
class ReleaseHeading:
    version: str
    line_index: int
    release_date: dt.date
    inline_previous_version: str | None


@dataclass(frozen=True)
class CompareLink:
    previous_version: str
    target: str


def _require_semver(value: str, description: str) -> None:
    if SEMVER_PATTERN.fullmatch(value) is None:
        raise ValueError(f"{description} {value!r} is not valid SemVer 2.0")


def _semver_precedence(
    value: str,
) -> tuple[int, int, int, tuple[tuple[int, int | str], ...]]:
    match = SEMVER_PATTERN.fullmatch(value)
    if match is None:
        raise ValueError(f"version {value!r} is not valid SemVer 2.0")

    prerelease_text = match.group(4)
    if prerelease_text is None:
        prerelease: tuple[tuple[int, int | str], ...] = ((2, 0),)
    else:
        prerelease = tuple(
            (0, int(identifier)) if identifier.isdigit() else (1, identifier)
            for identifier in prerelease_text.split(".")
        )
    return int(match.group(1)), int(match.group(2)), int(match.group(3)), prerelease


def _parse_date(value: str, description: str) -> dt.date:
    try:
        return dt.date.fromisoformat(value)
    except ValueError as error:
        raise ValueError(f"{description} has invalid release date {value!r}") from error


def _parse_compare_link(url: str, description: str) -> CompareLink:
    if url.startswith(COMPARE_URL_PREFIX):
        comparison = url.removeprefix(COMPARE_URL_PREFIX)
    elif url.startswith("compare/"):
        comparison = url.removeprefix("compare/")
    else:
        raise ValueError(
            f"{description} must use {COMPARE_URL_PREFIX}vPREVIOUS...vCURRENT"
        )

    if comparison.count("...") != 1:
        raise ValueError(f"{description} has a malformed comparison range")
    previous_tag, target_tag = comparison.split("...", maxsplit=1)
    if not previous_tag.startswith("v") or len(previous_tag) == 1:
        raise ValueError(f"{description} has a malformed previous tag")
    previous_version = previous_tag[1:]
    _require_semver(previous_version, f"{description} previous version")

    if target_tag == "HEAD":
        target = "HEAD"
    elif target_tag.startswith("v") and len(target_tag) > 1:
        target = target_tag[1:]
        _require_semver(target, f"{description} target version")
    else:
        raise ValueError(f"{description} has a malformed comparison target")

    return CompareLink(previous_version, target)


def _load_workspace_version(workspace: Path) -> str:
    manifest_path = workspace / "Cargo.toml"
    metadata = tomllib.loads(manifest_path.read_text(encoding="utf-8"))
    try:
        version = metadata["workspace"]["package"]["version"]
    except (KeyError, TypeError) as error:
        raise ValueError("Cargo.toml must define workspace.package.version") from error
    if not isinstance(version, str):
        raise ValueError("workspace.package.version must be a string")
    _require_semver(version, "workspace version")
    return version


def _parse_headings(
    lines: list[str],
) -> tuple[int, list[ReleaseHeading]]:
    unreleased_indices: list[int] = []
    headings: list[ReleaseHeading] = []
    versions_seen: set[str] = set()

    for index, line in enumerate(lines):
        if not line.startswith("## ["):
            continue
        if line == "## [Unreleased]":
            unreleased_indices.append(index)
            continue

        canonical = CANONICAL_HEADING_PATTERN.fullmatch(line)
        inline = INLINE_HEADING_PATTERN.fullmatch(line)
        if canonical is None and inline is None:
            raise ValueError(f"malformed changelog release heading on line {index + 1}")

        match = canonical if canonical is not None else inline
        assert match is not None
        version = match.group(1)
        _require_semver(version, f"changelog heading on line {index + 1}")
        if version in versions_seen:
            raise ValueError(f"duplicate changelog heading for version {version}")
        versions_seen.add(version)

        release_date = _parse_date(
            match.group(2 if canonical is not None else 3),
            f"changelog heading for {version}",
        )
        inline_previous: str | None = None
        if inline is not None:
            comparison = _parse_compare_link(
                inline.group(2), f"inline heading for {version}"
            )
            if comparison.target != version:
                raise ValueError(
                    f"inline heading for {version} compares to {comparison.target!r}"
                )
            inline_previous = comparison.previous_version

        headings.append(ReleaseHeading(version, index, release_date, inline_previous))

    if len(unreleased_indices) != 1:
        raise ValueError(
            "CHANGELOG.md must contain exactly one '## [Unreleased]' heading; "
            f"found {len(unreleased_indices)}"
        )
    return unreleased_indices[0], headings


def _parse_link_definitions(lines: list[str]) -> dict[str, tuple[int, str]]:
    definitions: dict[str, tuple[int, str]] = {}
    for index, line in enumerate(lines):
        match = LINK_DEFINITION_PATTERN.fullmatch(line)
        if match is not None:
            name, url = match.groups()
            if name in definitions:
                raise ValueError(f"duplicate changelog link definition [{name}]")
            definitions[name] = (index, url)
            continue

        if line.startswith("[Unreleased]"):
            raise ValueError(
                f"malformed [Unreleased] link definition on line {index + 1}"
            )
    return definitions


def _one_previous_version(candidates: Iterable[tuple[str, str]]) -> str:
    candidates = tuple(candidates)
    if not candidates:
        raise ValueError("could not determine the previous release version")

    versions = {version for _, version in candidates}
    if len(versions) != 1:
        details = ", ".join(f"{source}={version}" for source, version in candidates)
        raise ValueError(f"inconsistent previous release versions: {details}")
    return next(iter(versions))


def finalize(workspace: Path) -> FinalizedRelease:
    """Finalize CHANGELOG.md in-place after release-plz updates the workspace."""

    workspace = workspace.resolve()
    version = _load_workspace_version(workspace)
    changelog_path = workspace / "CHANGELOG.md"
    original = changelog_path.read_text(encoding="utf-8")
    had_final_newline = original.endswith("\n")
    lines = original.splitlines()

    unreleased_heading_index, headings = _parse_headings(lines)
    current_headings = [heading for heading in headings if heading.version == version]
    if len(current_headings) != 1:
        raise ValueError(
            f"CHANGELOG.md must contain exactly one heading for workspace version "
            f"{version}; found {len(current_headings)}"
        )
    current_heading = current_headings[0]

    ordered_headings: list[tuple[str, int]] = [
        ("Unreleased", unreleased_heading_index),
        *((heading.version, heading.line_index) for heading in headings),
    ]
    ordered_headings.sort(key=lambda item: item[1])
    current_position = next(
        index for index, (name, _) in enumerate(ordered_headings) if name == version
    )
    if (
        current_position == 0
        or ordered_headings[current_position - 1][0] != "Unreleased"
    ):
        raise ValueError(
            f"workspace release {version} must be the first release after Unreleased"
        )
    following_version = (
        ordered_headings[current_position + 1][0]
        if current_position + 1 < len(ordered_headings)
        else None
    )

    definitions = _parse_link_definitions(lines)
    if "Unreleased" not in definitions:
        raise ValueError("CHANGELOG.md must contain one [Unreleased] link definition")
    unreleased_link_index, unreleased_url = definitions["Unreleased"]
    unreleased_comparison = _parse_compare_link(
        unreleased_url, "[Unreleased] link definition"
    )
    if unreleased_comparison.target != "HEAD":
        raise ValueError("[Unreleased] link definition must compare to HEAD")

    current_definition = definitions.get(version)
    current_prefix_indices = [
        index for index, line in enumerate(lines) if line.startswith(f"[{version}]")
    ]
    expected_definition_indices = (
        {current_definition[0]} if current_definition is not None else set()
    )
    malformed_definition_indices = (
        set(current_prefix_indices) - expected_definition_indices
    )
    if malformed_definition_indices:
        malformed_index = min(malformed_definition_indices)
        raise ValueError(
            f"malformed [{version}] link definition on line {malformed_index + 1}"
        )

    current_comparison: CompareLink | None = None
    if current_definition is not None:
        _, current_url = current_definition
        current_comparison = _parse_compare_link(
            current_url, f"[{version}] link definition"
        )
        if current_comparison.target != version:
            raise ValueError(
                f"[{version}] link definition compares to {current_comparison.target!r}"
            )

    previous_candidates: list[tuple[str, str]] = []
    if current_heading.inline_previous_version is not None:
        previous_candidates.append(
            ("inline heading", current_heading.inline_previous_version)
        )
    if current_comparison is not None:
        previous_candidates.append(
            ("release link", current_comparison.previous_version)
        )
    if following_version is not None:
        previous_candidates.append(("previous heading", following_version))
    if unreleased_comparison.previous_version != version:
        previous_candidates.append(
            ("Unreleased link", unreleased_comparison.previous_version)
        )

    previous_version = _one_previous_version(previous_candidates)
    if _semver_precedence(previous_version) >= _semver_precedence(version):
        raise ValueError(
            f"previous release {previous_version} must precede "
            f"current release {version}"
        )
    if unreleased_comparison.previous_version not in {previous_version, version}:
        raise ValueError(
            f"[Unreleased] link starts at {unreleased_comparison.previous_version}; "
            f"expected {previous_version} or {version}"
        )

    canonical_heading = f"## [{version}] - {current_heading.release_date.isoformat()}"
    canonical_unreleased_link = f"[Unreleased]: {COMPARE_URL_PREFIX}v{version}...HEAD"
    canonical_release_link = (
        f"[{version}]: {COMPARE_URL_PREFIX}v{previous_version}...v{version}"
    )

    lines[current_heading.line_index] = canonical_heading
    lines[unreleased_link_index] = canonical_unreleased_link
    if current_definition is None:
        lines.insert(unreleased_link_index + 1, canonical_release_link)
    else:
        lines[current_definition[0]] = canonical_release_link

    finalized = "\n".join(lines)
    if had_final_newline:
        finalized += "\n"
    if finalized != original:
        changelog_path.write_text(finalized, encoding="utf-8")

    return FinalizedRelease(
        version=version,
        previous_version=previous_version,
        release_date=current_heading.release_date,
    )


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument(
        "--workspace",
        type=Path,
        default=Path(__file__).resolve().parent.parent,
    )
    arguments = parser.parse_args()

    try:
        release = finalize(arguments.workspace)
    except (OSError, ValueError, tomllib.TOMLDecodeError) as error:
        print(f"error: {error}", file=sys.stderr)
        return 1

    print(
        f"Finalized CHANGELOG.md for {release.version} "
        f"(previous release: {release.previous_version})."
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
