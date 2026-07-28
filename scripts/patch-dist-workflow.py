#!/usr/bin/env python3
"""Apply the small, audited hardening patch to cargo-dist's generated workflow.

`cargo-dist` remains the source of truth for the build matrix and artifact
plumbing. This adapter covers repository-policy gaps that cargo-dist 0.32.0
does not generate: least privilege, the exact public tag namespace, a
fail-closed custom release gate, a CycloneDX output typo, concurrency, and
immutable-safe announcement retries.

Every replacement is intentionally exact and single-use. A cargo-dist upgrade
therefore fails generation instead of silently dropping a policy guarantee.
Remove individual replacements as upstream cargo-dist gains equivalent output.
"""

from __future__ import annotations

from pathlib import Path
import sys


MARKER = "# SealTask hardening patch: least privilege and resumable announcement."


def replace_once(source: str, old: str, new: str, description: str) -> str:
    count = source.count(old)
    if count != 1:
        raise ValueError(f"expected one {description} block, found {count}")
    return source.replace(old, new, 1)


def main() -> int:
    if len(sys.argv) != 2:
        print(f"usage: {Path(sys.argv[0]).name} WORKFLOW", file=sys.stderr)
        return 2

    workflow = Path(sys.argv[1])
    source = workflow.read_text(encoding="utf-8")
    if MARKER in source:
        print(f"{workflow} is already hardened.")
        return 0

    source = replace_once(
        source,
        'permissions:\n  "contents": "write"\n',
        (
            'permissions:\n'
            '  "contents": "read"\n'
            f"\n{MARKER}\n"
            "concurrency:\n"
            "  group: oss-release-${{ github.ref }}\n"
            "  cancel-in-progress: false\n"
        ),
        "top-level permissions",
    )
    source = replace_once(
        source,
        "      - '**[0-9]+.[0-9]+.[0-9]+*'\n",
        "      - 'v[0-9]+.[0-9]+.[0-9]+*'\n",
        "release tag filter",
    )
    source = replace_once(
        source,
        "${{ steps.cargo-cyclonedx.output.paths }}",
        "${{ steps.cargo-cyclonedx.outputs.paths }}",
        "CycloneDX output",
    )
    source = replace_once(
        source,
        (
            "  host:\n"
            "    needs:\n"
            "      - plan\n"
            "      - build-local-artifacts\n"
            "      - build-global-artifacts\n"
        ),
        (
            "  host:\n"
            "    needs:\n"
            "      - plan\n"
            "      - custom-release-gate\n"
            "      - build-local-artifacts\n"
            "      - build-global-artifacts\n"
        ),
        "host release-gate dependency",
    )
    source = replace_once(
        source,
        (
            "    if: ${{ always() && needs.plan.result == 'success' && "
            "needs.plan.outputs.publishing == 'true' && "
            "(needs.build-global-artifacts.result == 'skipped' || "
            "needs.build-global-artifacts.result == 'success') && "
            "(needs.build-local-artifacts.result == 'skipped' || "
            "needs.build-local-artifacts.result == 'success') }}\n"
        ),
        (
            "    if: ${{ always() && needs.plan.result == 'success' && "
            "needs.custom-release-gate.result == 'success' && "
            "needs.plan.outputs.publishing == 'true' && "
            "(needs.build-global-artifacts.result == 'skipped' || "
            "needs.build-global-artifacts.result == 'success') && "
            "(needs.build-local-artifacts.result == 'skipped' || "
            "needs.build-local-artifacts.result == 'success') }}\n"
        ),
        "host release-gate condition",
    )
    announce_permissions = (
        '    permissions:\n'
        '      "attestations": "write"\n'
        '      "contents": "write"\n'
        '      "id-token": "write"\n'
    )
    if source.count(announce_permissions) != 1:
        raise ValueError("generated announce job did not retain write-only permissions")
    source = replace_once(
        source,
        (
            '          gh release create "${{ needs.plan.outputs.tag }}" '
            '--target "$RELEASE_COMMIT" $PRERELEASE_FLAG '
            '--title "$ANNOUNCEMENT_TITLE" '
            '--notes-file "$RUNNER_TEMP/notes.txt" artifacts/*\n'
        ),
        (
            '          if existing_release="$(gh release view '
            '"${{ needs.plan.outputs.tag }}" '
            '--json isDraft,isImmutable,targetCommitish 2>/dev/null)"; then\n'
            '            existing_target="$(jq -r \'.targetCommitish\' '
            '<<<"${existing_release}")"\n'
            '            if [[ "${existing_target}" != "${RELEASE_COMMIT}" ]]; then\n'
            '              echo "::error::Existing release targets '
            '${existing_target}, expected ${RELEASE_COMMIT}."\n'
            "              exit 1\n"
            "            fi\n"
            "            if jq -e '.isDraft == false' "
            '<<<"${existing_release}" >/dev/null; then\n'
            "              if ! jq -e '.isImmutable == true' "
            '<<<"${existing_release}" >/dev/null; then\n'
            '                echo "::error::Existing published release is not immutable; '
            'refusing to mutate it."\n'
            "                exit 1\n"
            "              fi\n"
            '              echo "Immutable release already exists; '
            'the verification job will prove its assets."\n'
            "            else\n"
            '              gh release upload "${{ needs.plan.outputs.tag }}" '
            "artifacts/* --clobber\n"
            '              gh release edit "${{ needs.plan.outputs.tag }}" '
            '--target "$RELEASE_COMMIT" '
            '--title "$ANNOUNCEMENT_TITLE" '
            '--notes-file "$RUNNER_TEMP/notes.txt" --draft=false\n'
            "            fi\n"
            "          else\n"
            '            gh release create "${{ needs.plan.outputs.tag }}" '
            '--target "$RELEASE_COMMIT" $PRERELEASE_FLAG '
            '--title "$ANNOUNCEMENT_TITLE" '
            '--notes-file "$RUNNER_TEMP/notes.txt" --verify-tag artifacts/*\n'
            "          fi\n"
        ),
        "GitHub release command",
    )

    workflow.write_text(source, encoding="utf-8")
    print(f"Hardened {workflow}.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
