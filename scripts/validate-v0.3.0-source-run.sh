#!/usr/bin/env bash
set -euo pipefail

REPOSITORY="sealtask/sealtask-oss"
RECOVERY_TAG="v0.3.0"
RECOVERY_TAG_OBJECT="34146df3c6b85793fc940c9b9bf5448ea8dc6d95"
RECOVERY_COMMIT="a0389a3aa6285b9bb136aa1756c11da7e6ec5a6e"
SOURCE_RUN_ID="30384064267"
SOURCE_WORKFLOW_ID="322259675"
SOURCE_REPOSITORY_ID="1312075908"

fail() {
  printf 'error: %s\n' "$*" >&2
  exit 1
}

[[ "${GITHUB_REPOSITORY:-}" == "${REPOSITORY}" ]] \
  || fail "recovery is restricted to ${REPOSITORY}"
[[ "${GITHUB_REF:-}" == "refs/heads/main" ]] \
  || fail "recovery must run from public main"
[[ "${IMMUTABLE_RELEASES_ENABLED:-}" == "true" ]] \
  || fail "immutable GitHub Releases must remain enabled"

tag_ref="$(gh api "repos/${REPOSITORY}/git/ref/tags/${RECOVERY_TAG}")"
printf '%s\n' "${tag_ref}" |
  jq -e \
    --arg tag_object "${RECOVERY_TAG_OBJECT}" \
    '
      .ref == "refs/tags/v0.3.0" and
      .object.type == "tag" and
      .object.sha == $tag_object
    ' >/dev/null

tag_object="$(gh api "repos/${REPOSITORY}/git/tags/${RECOVERY_TAG_OBJECT}")"
printf '%s\n' "${tag_object}" |
  jq -e \
    --arg commit "${RECOVERY_COMMIT}" \
    '
      .tag == "v0.3.0" and
      .object.type == "commit" and
      .object.sha == $commit
    ' >/dev/null

run="$(gh api "repos/${REPOSITORY}/actions/runs/${SOURCE_RUN_ID}")"
printf '%s\n' "${run}" |
  jq -e \
    --argjson run_id "${SOURCE_RUN_ID}" \
    --argjson workflow_id "${SOURCE_WORKFLOW_ID}" \
    --argjson repository_id "${SOURCE_REPOSITORY_ID}" \
    --arg sha "${RECOVERY_COMMIT}" \
    '
      .id == $run_id and
      .workflow_id == $workflow_id and
      .event == "push" and
      .status == "completed" and
      .conclusion == "failure" and
      .head_branch == "v0.3.0" and
      .head_sha == $sha and
      .path == ".github/workflows/release.yml" and
      .run_attempt == 2 and
      .repository.id == $repository_id and
      .head_repository.id == $repository_id and
      .repository.full_name == "sealtask/sealtask-oss" and
      .head_repository.full_name == "sealtask/sealtask-oss"
    ' >/dev/null

jobs="$(
  gh api \
    "repos/${REPOSITORY}/actions/runs/${SOURCE_RUN_ID}/jobs?per_page=100"
)"
printf '%s\n' "${jobs}" |
  jq -e \
    --arg sha "${RECOVERY_COMMIT}" \
    '
      def successful($name):
        ([.jobs[] |
          select(
            .name == $name and
            .conclusion == "success" and
            .head_sha == $sha and
            .run_attempt == 2
          )] | length) == 1;
      def skipped($name):
        ([.jobs[] |
          select(
            .name == $name and
            .conclusion == "skipped" and
            .head_sha == $sha and
            .run_attempt == 2
          )] | length) == 1;
      def step($job; $name; $conclusion):
        ([$job.steps[] |
          select(.name == $name and .conclusion == $conclusion)
        ] | length) == 1;

      successful("plan") and
      successful("custom-release-gate / validate") and
      successful("build-local-artifacts (aarch64-apple-darwin)") and
      successful("build-local-artifacts (x86_64-apple-darwin)") and
      successful("build-local-artifacts (x86_64-unknown-linux-gnu)") and
      successful("build-global-artifacts") and
      successful("host") and
      skipped("announce") and
      skipped("custom-verify-release") and
      (
        [.jobs[] |
          select(.name == "custom-publish-crates / Publish and verify crates.io")
        ] as $publishers |
        ($publishers | length) == 1 and
        $publishers[0].id == 90365621373 and
        $publishers[0].conclusion == "failure" and
        $publishers[0].head_sha == $sha and
        $publishers[0].run_attempt == 2 and
        step($publishers[0]; "Authenticate to crates.io with OIDC"; "success") and
        step($publishers[0]; "Publish the lockstep crate graph"; "success") and
        step($publishers[0]; "Install and verify the registry package"; "failure")
      )
    ' >/dev/null

artifacts="$(
  gh api \
    "repos/${REPOSITORY}/actions/runs/${SOURCE_RUN_ID}/artifacts?per_page=100"
)"
printf '%s\n' "${artifacts}" |
  jq -e \
    --argjson run_id "${SOURCE_RUN_ID}" \
    --arg sha "${RECOVERY_COMMIT}" \
    '
      def exact($id; $name; $digest):
        ([.artifacts[] |
          select(
            .id == $id and
            .name == $name and
            .digest == $digest and
            .expired == false and
            .workflow_run.id == $run_id and
            .workflow_run.head_sha == $sha
          )
        ] | length) == 1;

      .total_count == 7 and
      exact(
        8698437800;
        "artifacts-dist-manifest";
        "sha256:36c9d25ee2c4943dfd545be4005a5795cbabe1493ba760eade1502ebbd47c494"
      ) and
      exact(
        8698421602;
        "artifacts-build-global";
        "sha256:23acb75be0bffdff42ee35b3fd0ccd290b96aaf1fc39f7a92fd138f1365407d0"
      ) and
      exact(
        8698401425;
        "artifacts-build-local-x86_64-apple-darwin";
        "sha256:c6ef6d106a1534a389dfaa2a64fdf6a0baa4ddd3edff839a7d313b74c55ff091"
      ) and
      exact(
        8698332832;
        "artifacts-build-local-x86_64-unknown-linux-gnu";
        "sha256:8f8de2e59c15efa9c7f470f814fd4c18a814edf5c53a2a255e5522cf356b26bb"
      ) and
      exact(
        8698330697;
        "artifacts-build-local-aarch64-apple-darwin";
        "sha256:31b15234e0d0de05ffdd166d95b607a3be1800d005c5c94698f83026276a49a6"
      ) and
      exact(
        8698203151;
        "artifacts-plan-dist-manifest";
        "sha256:29f08311c4dfeba12d1cc65ca8d0b7c2d3c0fed47c382ba5a0f0a750990b6372"
      ) and
      exact(
        8698196033;
        "cargo-dist-cache";
        "sha256:1d9e56c91fddc5da17f5b8686f2578c31c42d9e4fdde8cc8c0607cc58ef56c7c"
      )
    ' >/dev/null

printf 'Validated the pinned v0.3.0 tag, source run, jobs, and artifacts.\n'
