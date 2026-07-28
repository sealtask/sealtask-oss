# Releasing SealTask OSS

The steady-state release operation is one reviewed pull request merge. Version
selection, changelog generation, exact dependency pins, generated CLI assets,
public mirroring, annotated tags, crates.io publication, cross-platform
artifacts, checksums, SBOMs, attestations, and post-publication verification are
automated.

```text
private master
  -> public main mirror
  -> automated private release PR
  -> reviewed merge
  -> atomic public main + annotated vX.Y.Z tag
  -> crates.io (OIDC, dependency order)
  -> immutable GitHub Release
  -> registry install + release/asset/attestation verification
```

The six crates are always versioned and published together in this order:

1. `sealtask-client-core`
2. `sealtask-client-auth`
3. `sealtask-client-crypto`
4. `sealtask-client-api`
5. `sealtask-client-runtime`
6. `sealtask`

## Steady-state procedure

1. Merge normal OSS changes to private `master`. Use Conventional Commit
   prefixes (`fix:`, `feat:`, and breaking-change markers) so release-plz can
   choose the next version and produce useful changelog entries.
2. Wait for **Prepare OSS Release** to create or update
   `release-plz-oss`. It also runs after every successful mirror and on weekday
   schedules, so a transient failure does not require a human kick.
3. Review the private release PR:
   - proposed lockstep version and `CHANGELOG.md`
   - all exact internal dependency pins
   - generated completions and man pages
   - the normal OSS checks
   - **OSS Release Platforms** on Linux and macOS
4. Merge that PR. This merge is the release approval.
5. Confirm the public **Release** workflow finishes. It must publish all six
   exact crate archives, install the registry copy of `sealtask`, create an
   immutable GitHub Release, and verify every downloaded asset and attestation.

Do not create or move release tags by hand. The private mirror workflow derives
the tag from the reviewed workspace version and atomically pushes public `main`
and the annotated tag. It then records an annotated private `oss-vX.Y.Z`
provenance tag at the exact monorepo source commit.

If there are no releasable commits, the release-PR workflow exits successfully
without changing its bot branch.

## One-time repository setup

These settings cannot safely be encoded in a workflow committed to either
repository. Configure them before merging the first automated release PR.

### Private repository

Install a narrowly scoped GitHub App that can create and update release PRs.
Grant it repository metadata read, contents read/write, pull requests
read/write. Pull requests write also authorizes the release-label operations
used by the workflow, so no separate issues permission is required. Allow it
to create/update the `release-plz-oss` branch under the repository's rules.

Protect private `master`: require pull requests and the repository's required
checks; reject direct pushes. Multi-maintainer repositories should also require
at least one human approval. A solo-maintainer repository must leave the
blanket approval count at zero because GitHub does not permit an author to
approve their own PR. Release approval remains independently enforced:
`release-plz-oss` is App-authored, so the maintainer can approve it, and the
mirror verifies that every new public release source came from a merged,
`release`-labeled PR whose head was that same-repository bot branch, whose
merge SHA is the exact tag source, and which has an approval on the final PR
head from a human who still has write or admin access, with no outstanding
authorized change request.

Store its credentials as private Actions secrets:

- `RELEASE_APP_ID`
- `RELEASE_APP_PRIVATE_KEY`

Set the private Actions repository variable `RELEASE_APP_LOGIN` to that App's
bot login, including the `[bot]` suffix.

Keep the existing `OSS_MIRROR_SSH_KEY` write deploy key restricted to
`sealtask/sealtask-oss`. Before adding or replacing that deploy key through
GitHub CLI, run `gh auth refresh --hostname github.com --scopes workflow` and
confirm `gh auth status --hostname github.com` lists `workflow`. GitHub retains
the creating OAuth authorization for deploy keys; without that scope, the key
cannot mirror files under `.github/workflows/`.

### Public repository

1. Enable immutable releases for `sealtask/sealtask-oss`. This must be enabled
   before the first automated GitHub Release. After verifying the setting, add
   the public Actions repository variable
   `IMMUTABLE_RELEASES_ENABLED=true`. This is the pre-publication activation
   latch; the post-publication verifier still proves the actual release is
   immutable.
2. Create a `crates-io` environment and restrict deployments to tags matching
   `v*`. Do not add another required reviewer if the reviewed private release PR
   is intended to be the only steady-state approval.
3. Create a `crates-io-bootstrap` environment with a required reviewer. Add a
   short-lived `CRATES_IO_BOOTSTRAP_TOKEN` environment secret only for the
   first publication.
4. Keep the default Actions token read-only. The generated release workflow
   escalates only the individual jobs that publish attestations or the GitHub
   Release.
5. Public `main` is mirror-only. Repository rules must reject deletion and
   non-fast-forward updates; ideally also restrict ordinary updates to the
   mirror deploy key or its replacement GitHub App. Add matching tag rules for
   `v*` that reject updates and deletion and allow creation only by the mirror
   identity. Protect private `oss-v*` provenance tags the same way.

## First automated release

All six crate names are initially unowned on crates.io, so trusted publishing
cannot be configured until the bootstrap publication establishes ownership.
The first release has one deliberate bootstrap detour:

1. Let **Prepare OSS Release** create the initial `v0.3.0` PR. This one-time
   transition moves the existing `0.2.1` workspace onto the new release
   contract.
2. Review and merge the PR normally. The mirror creates public `v0.3.0`.
3. The public **Release** workflow will build artifacts and stop at crates.io
   OIDC authentication because the trusted publishers do not exist yet. Do not
   create a GitHub Release manually.
4. In the public repository, run **Bootstrap crates.io ownership** with:
   - `release_tag`: `v0.3.0`
   - `confirmation`: `BOOTSTRAP-SEALTASK-CRATES`
5. After all six crates exist, configure this trusted publisher on each crate:

   ```text
   GitHub owner:        sealtask
   GitHub repository:   sealtask-oss
   Workflow filename:   release.yml
   Environment:         crates-io
   ```

   Configure it for all six names listed at the top of this document. The
   trusted workflow is the caller `release.yml`, not the reusable
   `publish-crates.yml`.
6. Revoke the bootstrap token and delete `CRATES_IO_BOOTSTRAP_TOKEN` from the
   environment.
7. Choose **Re-run failed jobs** on the original public **Release** run. The
   publisher checks the staged archive checksum against crates.io, skips each
   already-published crate, performs the registry install verification, and
   continues to the immutable GitHub Release.

Every later release uses OIDC and needs no crates.io token.

## Retry and recovery rules

The release systems are resumable but intentionally refuse ambiguous state.

| Failure | Safe action |
| --- | --- |
| Release PR says the public tree is behind | Wait for **Mirror OSS Workspace**, then rerun the release-PR job. |
| A crate publish times out or Cargo returns an uncertain result | Use **Re-run failed jobs**. The publisher resumes only when crates.io reports the exact staged SHA-256 checksum. |
| Some crates exist and later ones do not | Use **Re-run failed jobs**. Existing exact archives are skipped and publication continues in dependency order. |
| Public tag already points somewhere else | Stop. Never delete or move the tag; reconcile the source/history as an incident. |
| Public `main` has a non-mirror commit | The mirror rejects the non-fast-forward update. Reconcile public history explicitly; never force-push from the workflow. |
| A draft GitHub Release remains | Rerun the failed announcement job. It may refill that same draft and publish it. |
| A published release is not immutable | Stop. Automation refuses to mutate it; correct the repository setting and handle the release as an incident. |
| The immutable release already exists | Rerun only the failed verification job. Announcement validates the target and treats the immutable release as complete. |
| Asset or attestation verification fails | Rerun the verifier. Do not replace assets on an immutable release. |

GitHub Releases and crates.io are independent systems, so they cannot be one
transaction. The workflow publishes and verifies crates first, then publishes
the immutable GitHub Release. A retry-safe checksum state machine bridges that
boundary.

## Local validation

From `oss/`:

```bash
./scripts/check-release-metadata.sh workspace
python3 -m unittest scripts/test_check_release_metadata.py
python3 -m unittest scripts/test_finalize_release_changelog.py
python3 -m unittest scripts/test_prepare_initial_release.py
./scripts/test-release-plz-cargo.sh
./scripts/test-publish-crates.sh
./scripts/generate-release-workflow.sh check
cargo fmt --all --check
cargo check --workspace --all-targets --locked
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo test --workspace --all-targets --locked
```

`.github/workflows/release.yml` is generated by pinned cargo-dist 0.32.0; do
not edit it directly. `scripts/patch-dist-workflow.py` is a fail-closed adapter
for the small policy gaps in that generated version. The generation check
rebuilds the workflow in a clean temporary Git repository, applies each exact
single-use patch, and compares the result byte-for-byte. Remove a patch when a
future pinned cargo-dist version emits the equivalent guarantee itself.

Cargo-dist normally rejects any edited generated workflow at runtime. The
checked-in `allow-dirty = ["ci"]` disables only that redundant runtime
comparison so the audited hardening can run. It does not waive drift checks:
`generate-release-workflow.sh` removes the exception in its temporary
repository, deletes the copied workflow, regenerates it from scratch, applies
the patch, compares it with the checked-in file, restores the runtime config,
and finally rehearses both pull-request planning and tagged release planning.

Pinned release-plz 0.3.160 similarly hard-codes
`cargo package --allow-dirty --workspace` while reconstructing a prior
git-only release. `scripts/release-plz-cargo.sh` forwards every Cargo call
unchanged except that exact command, where it excludes the two unpublished
StrongBox browser-WASM packages. The adapter fails closed if release-plz
changes the command shape; remove it when upstream can scope the snapshot to
the release-managed package graph.

After committing a release candidate in a temporary standalone checkout, this
packages all six archives without registry access or publication:

```bash
DRY_RUN=1 ./scripts/publish-crates.sh
```

For an emergency token-based rerun from an exact annotated tag, set
`CARGO_REGISTRY_TOKEN` and `RELEASE_TAG=vX.Y.Z`, then run
`./scripts/publish-crates.sh`. Prefer the protected hosted workflows because
they preserve approval and provenance evidence.

From the private monorepo root, the source/tag/review state machines have
network-free regression suites:

```bash
python3 -m unittest scripts/test_release_review_policy.py
./scripts/test-resolve-oss-release-tag.sh
./scripts/test-persist-oss-release-baseline.sh
./scripts/test-push-oss-subtree.sh
```
