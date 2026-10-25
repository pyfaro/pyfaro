# Publishing Pyfaro

The latest published version is [`0.4.0-alpha.1`](https://crates.io/crates/pyfaro/0.4.0-alpha.1).
It adds `dev`, named tasks, and the `create fastapi` preset to the commands of
`0.3.0-alpha.1`. `init --apply` remains Linux-only; the other commands are
tested on Linux, macOS, and Windows. The repository now develops unpublished
`0.5.0-beta.1` (v0.5.0 Public Beta work). Its version number is not a
publication approval.

Completing the `v0.1.0 — Foundation` or `v0.2.0 — Project Management` backlog
milestones means their source-work criteria are met, not that a stable crate
has shipped. CI package and publish dry-runs do not upload a crate. Before any
future publication, the maintainer must explicitly approve the exact version
and merged commit after reviewing the archive and CI evidence. A preparation
PR alone is not publication approval.

## How to release

Releases go through the manual [Release workflow](#manual-release-workflow).
The steps below assume `<version>` is the development version already in
`Cargo.toml` (for example `0.4.0-alpha.1`) and that CI on `main` is green.

1. **Prepare.** In a PR, rename `## Unreleased — <version> development` in
   `CHANGELOG.md` to `## <version> — YYYY-MM-DD` and replace "This version
   has not been published" with a one-line introduction. That section becomes
   the GitHub Release notes. Merge the PR once CI passes.
2. **Dry run (optional).** Check, package, and build everything without
   uploading:

   ```sh
   gh workflow run release.yml -f version=<version>
   gh run watch "$(gh run list --workflow release.yml --limit 1 --json databaseId --jq '.[0].databaseId')"
   ```

   Download the `crate` artifact to review the archive and its SHA-256, and
   the `binaries` artifact for the five release archives. The run fails if
   any target does not build, reports the wrong version, or does not build
   to identical bytes twice.
3. **Publish.** Start the real run from `main`:

   ```sh
   gh workflow run release.yml -f version=<version> -f publish=true
   ```

   Preflight, CI on all three platforms, the audit, packaging, and the
   reproducible builds of all five binaries run first. Nothing reaches
   crates.io until all of them pass.
4. **Approve.** The **Publish to crates.io** job waits on the `crates-io`
   environment. Approve it in the run page (**Review deployments**), or:

   ```sh
   run=<run id>
   env_id=$(gh api "repos/pyfaro/pyfaro/actions/runs/$run/pending_deployments" --jq '.[0].environment.id')
   gh api -X POST "repos/pyfaro/pyfaro/actions/runs/$run/pending_deployments" \
     -F "environment_ids[]=$env_id" -f state=approved -f comment="Approve <version>"
   ```

   The workflow then uploads, tags `v<version>`, attests and publishes the
   release assets as a GitHub Release, and reinstalls the version from
   crates.io.
5. **Verify.** Check the crates.io page, the tag, and the release assets:

   ```sh
   gh release download v<version> -p 'SHA256SUMS' -p '*x86_64-unknown-linux-gnu.tar.gz'
   sha256sum -c --ignore-missing SHA256SUMS
   gh attestation verify pyfaro-<version>-x86_64-unknown-linux-gnu.tar.gz --repo pyfaro/pyfaro
   cargo install pyfaro --version <version> --locked
   ```

6. **Record.** In a follow-up PR, add the publication paragraph to the
   changelog section and a "Published release" record below, point the README
   install command at `<version>`, move `Cargo.toml` to the next development
   version, and add a new `## Unreleased — <next> development` section.

If preflight fails, fix the reported input (wrong ref, version mismatch,
missing changelog heading, existing tag, or a version crates.io already
knows) and start again. If a failure happens after the upload, see the
recovery notes in the workflow sections below. Published versions can never
be replaced; a defective one is yanked and superseded by a new version.

## Published release: 0.4.0-alpha.1

- Version: [`pyfaro 0.4.0-alpha.1`](https://crates.io/crates/pyfaro/0.4.0-alpha.1).
- Source: merged [PR #131](https://github.com/pyfaro/pyfaro/pull/131), commit
  [`107704e8e4e4f7c728f905e77e7e5ce5531c8880`](https://github.com/pyfaro/pyfaro/commit/107704e8e4e4f7c728f905e77e7e5ce5531c8880).
- Registry timestamp: `2026-10-25T10:10:09.854629Z`.
- Published archive SHA-256:
  `3280c65c0bb99d1af90e79bb7c7e325958400cd5e227f3bd41cbdf40484ba74d`.
  It is identical to the reviewed `crate` artifact of the run.
- Release workflow
  [run 39950335350](https://github.com/pyfaro/pyfaro/actions/runs/39950335350),
  dispatched from `main` with `publish` checked. The checks ran on
  2026-10-23 and the maintainer approved the `crates-io` environment
  deployment on 2026-10-25. It was published through crates.io trusted
  publishing (GitHub provider), with no long-lived token.
- Every job passed: preflight, CI on Linux, macOS, and Windows (including the
  end-to-end tests), the RustSec audit, package, publish, install from
  crates.io, and GitHub Release.
- The workflow created the annotated [`v0.4.0-alpha.1` tag](https://github.com/pyfaro/pyfaro/tree/v0.4.0-alpha.1)
  on the source commit above. The [GitHub prerelease](https://github.com/pyfaro/pyfaro/releases/tag/v0.4.0-alpha.1)
  was published at `2026-10-25T10:11:48Z`. It attaches standalone archives
  for x86_64 and aarch64 Linux, aarch64 and x86_64 macOS, and x86_64 Windows,
  the `.crate`, and `SHA256SUMS`. It has no build attestations; those start
  with `0.5.0-beta.1`.
- After release, the downloaded Linux x86_64 archive and the `.crate` passed
  `sha256sum -c`, and the binary reported `pyfaro 0.4.0-alpha.1`. With that
  binary and uv 0.9.26, `create fastapi`, `sync`, `check` (ok), and `test`
  (2 passed) worked.

This version exists on crates.io and must not be published again.

## Published release: 0.3.0-alpha.1

- Version: [`pyfaro 0.3.0-alpha.1`](https://crates.io/crates/pyfaro/0.3.0-alpha.1).
- Source: merged [PR #100](https://github.com/pyfaro/pyfaro/pull/100), commit
  [`2f041243b9d74cefaa526f017ef84118f95e50a0`](https://github.com/pyfaro/pyfaro/commit/2f041243b9d74cefaa526f017ef84118f95e50a0).
- Registry timestamp: `2026-10-22T09:50:14.928095Z`.
- Published archive SHA-256:
  `67f43a670e90536a00ed55be57524ce094baba837955408ea71e6163ee6922fe`.
  The workflow compared the registry checksum with the reviewed artifact.
- This was the first release made through the Release workflow,
  [run 39757664060](https://github.com/pyfaro/pyfaro/actions/runs/39757664060),
  dispatched from `main` with `publish` checked. The maintainer approved the
  `crates-io` environment deployment. It was published through crates.io
  trusted publishing (GitHub provider), with no long-lived token.
- Every job passed: preflight, CI on Linux, macOS, and Windows, the RustSec
  audit, package (all-target tests from the unpacked archive and a publish
  dry-run), publish, install from crates.io, and GitHub Release.
- The workflow created the annotated [`v0.3.0-alpha.1` tag](https://github.com/pyfaro/pyfaro/tree/v0.3.0-alpha.1)
  on the source commit above. The [GitHub prerelease](https://github.com/pyfaro/pyfaro/releases/tag/v0.3.0-alpha.1)
  was published at `2026-10-22T09:51:48Z`. It attaches standalone archives
  for x86_64 and aarch64 Linux, aarch64 and x86_64 macOS, and x86_64 Windows,
  the `.crate`, and `SHA256SUMS`.
- After release, the downloaded Linux x86_64 archive and the `.crate` passed
  `sha256sum -c`, and the binary reported `pyfaro 0.3.0-alpha.1`. With that
  binary and uv 0.9.26, `create lib`, `sync`, `check` (ok), and `test`
  (1 passed) worked.

This version exists on crates.io and must not be published again.

## Published release: 0.2.0-alpha.1

- Version: [`pyfaro 0.2.0-alpha.1`](https://crates.io/crates/pyfaro/0.2.0-alpha.1).
- Source: merged [PR #89](https://github.com/pyfaro/pyfaro/pull/89), commit
  [`c7d185c8c0a247396104b183c8473bddcd299856`](https://github.com/pyfaro/pyfaro/commit/c7d185c8c0a247396104b183c8473bddcd299856).
- Registry timestamp: `2026-10-22T08:37:08.585547Z`.
- Verified publisher and current owner: `mhbxyz`.
- Published archive SHA-256:
  `15c76a1da74c8114d0d3a8b3bb16c03a5985251f5703c94c66960bb2ab9bda37`.
- The registry checksum matches the locally packaged archive.
- Before upload, a clean checkout of the source commit passed `cargo fmt`,
  Clippy with `-D warnings`, all-target tests (124 passed), a release build,
  `cargo audit`, `cargo package`, and a publish dry-run. All-target tests
  also passed (124) from the unpacked package.
- `cargo install pyfaro --version 0.2.0-alpha.1 --locked` succeeded from
  crates.io. The installed binary reported `pyfaro 0.2.0-alpha.1`. Linux
  smoke checks with uv 0.9.26 covered `init` preview/apply/reapply (the
  reapply left output byte-identical), `sync`, `run`, `tools`, `doctor`,
  `create lib`, and `create cli`.
- The annotated [`v0.2.0-alpha.1` tag](https://github.com/pyfaro/pyfaro/tree/v0.2.0-alpha.1)
  resolves to the source commit above, not a later development commit.
  The [GitHub prerelease](https://github.com/pyfaro/pyfaro/releases/tag/v0.2.0-alpha.1) was published at
  `2026-10-22T09:05:49Z` by the GitHub Release workflow, at maintainer
  request. It attaches standalone archives for five targets, the `.crate`
  downloaded from crates.io and checked against the registry checksum, and
  `SHA256SUMS`. The downloaded Linux x86_64 archive matched its checksum and
  reported `pyfaro 0.2.0-alpha.1`.

This version exists on crates.io and must not be published again.

## Published release: 0.1.0-alpha.2

- Version: [`pyfaro 0.1.0-alpha.2`](https://crates.io/crates/pyfaro/0.1.0-alpha.2).
- Source: merged [PR #78](https://github.com/pyfaro/pyfaro/pull/78), commit
  [`be259107668c1a34dfd52d24ec10fa14a8563cbb`](https://github.com/pyfaro/pyfaro/commit/be259107668c1a34dfd52d24ec10fa14a8563cbb).
- Registry timestamp: `2026-10-19T05:58:23.344503Z`.
- Verified publisher and current owner: `mhbxyz`.
- Published archive SHA-256:
  `e52fd92c157227330e3894f55abd7b171317cd3788ec5596f9bd12aabf5fbfd1`.
- The downloaded archive matches both the registry checksum and the locally
  verified package. Its VCS metadata records the source commit above.
- `cargo install pyfaro --version 0.1.0-alpha.2 --locked` succeeded from
  crates.io. The installed binary reported the exact version and passed
  offline Linux `init` preview/apply/reapply and `sync`, `run`, and `tools`
  smoke checks with uv 0.9.26. Reapplying preserved `pyproject.toml` and
  `uv.lock` byte-for-byte.
- The annotated [`v0.1.0-alpha.2` tag](https://github.com/pyfaro/pyfaro/tree/v0.1.0-alpha.2)
  resolves to the source commit above, not a later development
  commit. The [GitHub Release](https://github.com/pyfaro/pyfaro/releases/tag/v0.1.0-alpha.2)
  was published as a prerelease at `2026-10-19T06:19:32Z` after separate
  maintainer authorization. It has no attached standalone binaries or
  installer artifacts.

The preparation and post-merge verification are tracked in
[issue #77](https://github.com/pyfaro/pyfaro/issues/77); the manual tag and
GitHub prerelease are tracked in
[issue #81](https://github.com/pyfaro/pyfaro/issues/81). This version exists
on crates.io and must not be published again.

## First manual alpha: 0.1.0-alpha.1

The first crates.io alpha was published on **2026-10-19**, ahead of the public-beta
milestone. It contains the useful but experimental [`init`](init.md) workflow.
This does not move roadmap milestones or expand B06: automated distribution,
standalone artifacts, and GitHub Release automation still belong to B02/B06
for beta.
The first alpha supports `init --apply` on Linux only, as explicitly agreed
for the early release. macOS and Windows retain compilation and previews;
their application workflow is deferred pending process-lifecycle guarantees.

Publication stayed disabled during bootstrap. With the initialization slice
implemented and tested, the release-preparation PR restricts publication to
`publish = ["crates-io"]`. This permits verification and manual uploads;
each upload still requires explicit authorization. No workflow uploads this crate.

For every future version, **do not run an uploading `cargo publish` until the
maintainer explicitly approves that release and commit.** Merging a preparation
PR, passing CI, or accepting a dry-run is not that approval.

### Verified publication record

- Version: [`pyfaro 0.1.0-alpha.1`](https://crates.io/crates/pyfaro/0.1.0-alpha.1).
- Source: merged [PR #68](https://github.com/pyfaro/pyfaro/pull/68), commit
  [`5ef04f55aa8c91778cc920ab6fe0d6b3c0a2921f`](https://github.com/pyfaro/pyfaro/commit/5ef04f55aa8c91778cc920ab6fe0d6b3c0a2921f).
- Registry timestamp: `2026-10-19T03:32:12.263591Z`.
- Verified publisher and current owner: `mhbxyz`.
- Published archive SHA-256:
  `9de5d69bdd76afc56661c444f07ab78f99050eb60f26b9730f2bcab00180b8f3`.
- The downloaded archive matches the registry checksum, records the source
  commit above, and contains the reviewed package files.
- `cargo install pyfaro --version 0.1.0-alpha.1 --locked --root <temporary-prefix>`
  succeeded. The installed binary reported the exact version and passed
  offline new/existing-project initialization, preview preservation, and
  idempotence checks with uv 0.9.26 on Linux.

This version already exists and must not be published again. No release tag
or GitHub Release was created for `alpha.1`.

### Name and ownership

On 2026-10-15, the official crates.io API returned HTTP 404 with
`crate pyfaro does not exist` for:

<https://crates.io/api/v1/crates/pyfaro>

The name was unregistered at that check, not reserved. The first publication
subsequently registered it under `mhbxyz`. Confirm the publishing account and
owner permissions before future uploads; credentials remain local and must
not be committed to this repository.

## Pre-publication verification for a future release

First agree on the next release version and prepare it in a reviewed PR.
Do not publish the repository's `0.5.0-beta.1` development version merely
because it is present in `Cargo.toml`. Use a clean checkout of the merged
preparation commit, not the PR branch, and the pinned Rust toolchain. Install
uv 0.9.26 separately and provide Python 3.10+ (CI uses 3.12). Repeat these
commands after merge and record the commit and resulting archive checksum
with the release issue before requesting publication approval. The following
Unix shell block stops at the first failure. On Windows, run each command
separately in PowerShell and stop if `$LASTEXITCODE` is nonzero. No publishing
token is needed:

```sh
set -e
cargo fmt --all -- --check
cargo clippy --locked --all-targets --all-features -- -D warnings
cargo test --locked --all-targets
cargo build --locked
cargo build --locked --release
cargo audit
cargo package --locked --list
cargo package --locked
cargo publish --dry-run --locked
```

Review the package inventory: Rust sources, test fixtures, Cargo manifests
and lockfile, license, changelog, README, contributor/security documents,
and all command/release/architecture docs are explicitly included.
Development graph/cache state, GitHub/backlog automation, and build outputs
must not enter the archive. Cargo also adds its normalized/original manifests
and VCS information. Inspect `target/package/pyfaro-<approved-version>.crate`
using the exact version from `Cargo.toml`, and record its SHA-256 checksum
outside the package. Do not edit the package afterward without repeating
the verification.

On Linux, smoke-test a dependency-free new project and idempotent application
with uv. `init --apply` is intentionally unsupported on macOS and Windows:

```sh
set -e
smoke_project="$(mktemp -d)/project"
cargo run --locked -- init "$smoke_project"
cargo run --locked -- init "$smoke_project" --apply --offline
before=$(sha256sum "$smoke_project/pyproject.toml" "$smoke_project/uv.lock")
cargo run --locked -- init "$smoke_project" --apply --offline
test "$before" = "$(sha256sum "$smoke_project/pyproject.toml" "$smoke_project/uv.lock")"
```

The final comparison must succeed: reapplying must leave both project files
byte-for-byte unchanged.

On Linux or macOS, start from a fresh disposable path and smoke-test the
other commands against a dependency-free project. Stop at the first failure:

```sh
set -e
smoke_project="$(mktemp -d)/project"
uv init --bare "$smoke_project"
cargo run --locked -- sync --project "$smoke_project" --offline
cargo run --locked -- run --project "$smoke_project" --locked --offline -- python --version
cargo run --locked -- tools uv
cargo run --locked -- tools --project "$smoke_project"
cargo run --locked -- python list --only-installed --offline
cargo run --locked -- doctor --project "$smoke_project"
```

On Windows, run the equivalent commands in PowerShell with an exit-status
check after each external process:

```powershell
function Invoke-Checked([scriptblock]$Command) {
    & $Command
    if ($LASTEXITCODE -ne 0) { throw "Release check failed: $LASTEXITCODE" }
}
$smokeProject = Join-Path ([IO.Path]::GetTempPath()) ('pyfaro-release-' + [guid]::NewGuid().ToString('N'))
Invoke-Checked { uv init --bare $smokeProject }
Invoke-Checked { cargo run --locked -- sync --project $smokeProject --offline }
Invoke-Checked { cargo run --locked -- run --project $smokeProject --locked --offline -- python --version }
Invoke-Checked { cargo run --locked -- tools uv }
Invoke-Checked { cargo run --locked -- tools --project $smokeProject }
Invoke-Checked { cargo run --locked -- python list --only-installed --offline }
Invoke-Checked { cargo run --locked -- doctor --project $smokeProject }
```

Each block uses a fresh temporary project path. Remove its directory after
recording the evidence; keep it temporarily if a check fails for diagnosis.

The commands should exit successfully. `sync` creates `.venv` and `uv.lock`,
`run` prints the interpreter version, and `tools uv` reports the selected uv
path and version. The all-tool listing and doctor may mark optional Ruff, ty,
or pytest as unavailable without failing or installing them. Creation,
dependency editing, Python pinning, and doctor failure paths are covered by
the all-target integration suite; test a generated library and CLI manually
before any future publication approval.

Also run `cargo test --locked --all-targets` from the unpacked package to
verify the shipped tests and fixtures are self-contained. Check the
`--version` output matches the approved release version. Review the candidate
changelog and CI evidence on the preparation PR: full Rust checks on Linux,
macOS, and Windows, plus the RustSec audit. Do not close the preparation issue
until the merged archive and checksum have been verified.

## Only after explicit publication approval

1. Confirm the clean commit, version availability, crates.io account, owner
   permissions, successful checks, and version with the maintainer. Authenticate
   locally through Cargo; do not put a token in the repository or logs.
2. Publish the approved commit manually:

   ```sh
   cargo publish --locked
   ```

3. Verify the crates.io entry and install that exact published version with
   `cargo install pyfaro --version <approved-version> --locked --root <temporary-prefix>`
   in a disposable location, substituting concrete paths and version.

4. Confirm the resulting binary's version and smoke-test `init`, `sync`,
   `run`, `tools`, `create`, `add`, `remove`, `python`, and `doctor` with the
   external uv/Python requirements. Record the actual publication date and
   registry checksum in a follow-up reviewed PR. Update the changelog,
   release record, README, and development package version.
5. Create/push a release tag and, if requested, a manual GitHub Release only
   after publication is verified and each action is separately authorized.
   Point the tag to the exact published source commit, not a later development
   commit. Mark alpha GitHub Releases as prereleases and do not claim
   standalone artifacts unless they have been built and verified. The
   workflows below automate these steps.

A published crate version cannot be overwritten. If a defective alpha ships,
agree on yanking it and publishing a corrected alpha with a new version;
never retag or silently reuse a published version. Rotate/revoke credentials
if compromised.

## Manual release workflow

`.github/workflows/release.yml` automates the publication steps above. It
only runs when started by hand (**Actions → Release → Run workflow**, or
`gh workflow run release.yml -f version=<version> -f publish=true`) from
`main`. It does not replace the approval gate: merging the preparation PR
first sets the version and dates its `## <version>` changelog section.

1. **Preflight** rejects a non-`main` ref, a version that differs from
   `Cargo.toml`, a missing `## <version>` changelog heading, an existing
   `v<version>` tag, and a version already known to crates.io.
2. **Checks** and **Audit** reuse the CI and Security workflows on all three
   platforms.
3. **Package** builds the archive, checks that it is `pyfaro <version>`
   packaged from the workflow's clean commit, runs all-target tests from the
   unpacked package, runs `cargo publish --dry-run`, and keeps the `.crate`
   and its SHA-256 as the `crate` workflow artifact for review.
4. **Binaries** (in parallel with 2 and 3) builds the five release archives
   twice and requires identical bytes, as described in
   [Reproducible builds](#reproducible-builds). It keeps the verified
   archives as the `binaries` artifact.
5. **Publish** runs only with `publish` checked and only after 2, 3, and 4
   passed, so a broken target blocks the upload. It waits for approval on the
   `crates-io` environment, obtains a short-lived token through crates.io
   trusted publishing, uploads, and fails if the registry checksum differs
   from the reviewed archive. It then creates the annotated `v<version>` tag
   on the exact workflow commit.
6. **GitHub Release** runs unless `github-release` is unchecked. It calls
   `.github/workflows/github-release.yml` with the `binaries` artifact from
   step 4, described below.
7. **Install from crates.io** installs the published version with `--locked`
   and checks its `--version` output.

Run it once with `publish` unchecked to obtain the archive and checksum for
review, then run it again with `publish` checked after approval. Afterward,
record the release in a follow-up PR as for earlier versions and move the
development version forward.

Releases are dispatched, not started by pushing a tag, and that is
deliberate: the workflow creates the tag only after the crates.io upload is
verified, so a `v<version>` tag always means a published version. This
dispatch-then-tag pipeline is how the "tag-to-release" requirement is met:
creating the tag leads straight to its GitHub Release, whose assets were
built and compared before the upload and are attested.

The versions have to agree before anything ships. The workflow input must
equal the `Cargo.toml` version, the packaged `.crate` must be `pyfaro
<version>` cut from the released commit, the tag is `v<version>`, every
binary that can run on its builder must print `pyfaro <version>`, and the
changelog needs a `## <version>` section for the release notes.

One-time setup is described in [Ownership and credentials](#ownership-and-credentials).

If the upload succeeds but tagging fails, do not rerun the workflow:
preflight rejects the published version. Create the missing tag on the
workflow's commit, following the approval steps above, then run the GitHub
Release workflow for it.

### GitHub Release workflow

`.github/workflows/github-release.yml` attests the release assets of a
version that is already on crates.io and tagged, and creates its GitHub
Release. The Release workflow calls it after publishing, handing over the
archives it already built and compared. Run it by hand to backfill an older
tag or refresh its assets; it then builds the tagged source itself with the
same reusable [build workflow](#reproducible-builds):

```sh
gh workflow run github-release.yml -f tag=v0.2.0-alpha.1
```

It checks that the tag exists and the version is on crates.io, then the
`assemble` job:

1. requires exactly one archive for each target below;
2. downloads the `.crate` from crates.io, checks it against the registry
   checksum, and checks that it is `pyfaro <version>` packaged from the
   tag's commit;
3. creates a signed build provenance attestation for every archive and the
   `.crate`, then verifies each one with `gh attestation verify`;
4. writes a `SHA256SUMS` file for every asset.

A separate job with `contents: write` creates the release from the attested
assets, so the job that can sign has no repository write access.

| Target | Runner | Archive |
| --- | --- | --- |
| `x86_64-unknown-linux-gnu` | Ubuntu 22.04 | `.tar.gz` |
| `aarch64-unknown-linux-gnu` | Ubuntu 22.04 Arm | `.tar.gz` |
| `aarch64-apple-darwin` | macOS | `.tar.gz` |
| `x86_64-apple-darwin` | macOS, cross-compiled | `.tar.gz` |
| `x86_64-pc-windows-msvc` | Windows | `.zip` |

Each archive holds the `pyfaro` executable, `README.md`, `LICENSE`, and
`CHANGELOG.md`.

The notes are the `## <version>` section of `CHANGELOG.md` from the
triggering ref (the released commit, or `main` for a backfill), followed by
install instructions and GitHub's generated list of merged pull requests
since the previous release. Versions with a pre-release suffix become
prereleases. When the release already exists, its notes are rebuilt the
same way, including a regenerated pull request list, and its assets are
re-uploaded. Releases cut before this pipeline landed have no attestations;
backfilling one attaches freshly built, attested archives that differ from
the originals.

## Reproducible builds

`.github/workflows/build-binaries.yml` is the one place the release binaries
are built. The Release workflow runs it before the crates.io upload, GitHub
Release runs it when backfilling a tag, and the **Release build check**
workflow runs it on every pull request that touches the release pipeline, so
changes to the pipeline are exercised before a release depends on them.

For each target it builds **twice**, in two jobs on separate fresh runners
with the checkout in different directories, and a final job requires the
SHA-256 of every binary and archive to match, failing otherwise. The step
summary lists both hashes for each target. The build itself is
`.github/scripts/release_artifacts.py`:

- `cargo build --locked --release` with the pinned Rust 1.90.0 toolchain,
  `CARGO_INCREMENTAL=0`, and `--remap-path-prefix` for the checkout, Cargo
  home, and rustup home, so no runner path ends up in the binary. Windows
  also links with `/Brepro`, which removes the linker's build timestamp.
- `SOURCE_DATE_EPOCH` is the commit time of the built commit.
- The archive is written by the script, not by `tar` or `7z`: entries are
  sorted, owners are `0`/empty, modes are `0644` (and `0755` for the
  executable), every timestamp is the commit time, and the gzip header has
  no name or time.
- Each archive's `--version` is checked against `pyfaro <version>` where the
  binary can run. The Intel macOS binary is cross-compiled on an Arm runner
  and checked under Rosetta 2 (`arch -x86_64`). If a runner ever lacks
  Rosetta, the check is skipped with a workflow notice that says why rather
  than failing the release.

### Reproduce an archive

To check that a published archive is what the tagged source builds, on a
machine like the release runner (Ubuntu 22.04 for the Linux archives):

```sh
git clone --branch v<version> https://github.com/pyfaro/pyfaro.git
cd pyfaro
rustup toolchain install 1.90.0 --target <target>
python3 .github/scripts/release_artifacts.py build --target <target>
gh release download v<version> --repo pyfaro/pyfaro -p SHA256SUMS
(cd dist && sha256sum -c --ignore-missing ../SHA256SUMS)
```

Run it from a clean checkout; it refuses to build with modified tracked
files. It needs Python 3.11 or newer. It prints the SHA-256 of the binary
and of the archive and writes the archive to `dist/`; `sha256sum -c` then
compares it with the matching line of `SHA256SUMS`.

Bit-identical output also needs an equivalent toolchain and system: the
same Rust version and the same linker, C library, and (on macOS) SDK as the
runner image. If only the archive hash differs, extract the `pyfaro`
executable from your archive and from the published one and compare them
with `sha256sum`: the binary is the stricter test, and a different zlib can
compress the same files to different bytes. The workflow logs print
`rustc -vV` for each build. Releases cut before this pipeline landed (see its
`CHANGELOG.md` entry) were not built reproducibly and have no script.

## Verifying a download

Every archive and the `.crate` of a release made after this pipeline landed
has a [GitHub artifact attestation](https://docs.github.com/en/actions/security-guides/using-artifact-attestations-to-establish-provenance-for-builds):
a Sigstore-signed SLSA build provenance statement that names the repository,
workflow, and commit that produced the file. Check a download with the
GitHub CLI:

```sh
gh attestation verify pyfaro-<version>-<target>.tar.gz --repo pyfaro/pyfaro
```

Success means the file's digest matches an attestation created by a
workflow in `pyfaro/pyfaro`. The attestation is signed with a short-lived
certificate tied to that workflow run, so there is no signing key to protect
or rotate. `SHA256SUMS` additionally guards against corrupted downloads:

```sh
sha256sum -c --ignore-missing SHA256SUMS
```

## Ownership and credentials

The release process has one human gate and no stored secrets.

| What | Who or what |
| --- | --- |
| Approve the `crates-io` environment | Required reviewer: `mhbxyz` (the only one). Self-review is allowed, so the maintainer can approve their own dispatch. |
| Branches that can deploy to `crates-io` | Protected branches only (`main`) |
| crates.io owners of `pyfaro` | `mhbxyz` (user owner; no team owners) |
| Repository secrets, environment secrets, variables | None |
| Default `GITHUB_TOKEN` permissions | Read-only; Actions cannot approve pull requests |

**Trusted publishing.** crates.io is configured (crate settings, *Trusted
Publishing*) to accept uploads from GitHub repository `pyfaro/pyfaro`,
workflow file `release.yml`, environment `crates-io`. In the publish job,
`rust-lang/crates-io-auth-action` exchanges the run's OIDC token for a
crates.io token that is valid only for that run's upload. No registry token,
signing key, or other credential is stored in GitHub or in this repository.
Only the `publish` job requests `id-token`
for crates.io; renaming `release.yml` or the environment breaks publishing
until the trusted publisher is updated to match.

**Workflow permissions.** Every workflow starts with `contents: read`; jobs
get more only where noted, and the `GITHUB_TOKEN` is the only token used:

| Job | Extra permissions | Why |
| --- | --- | --- |
| Release / Publish to crates.io | `contents: write`, `id-token: write` | Create the tag; trusted publishing |
| Release / Install from crates.io | none (`permissions: {}`) | It only runs `cargo install` |
| Release / GitHub Release (caller) | `contents: write`, `id-token: write`, `attestations: write` | Ceiling for the called jobs below |
| GitHub Release / Assemble and attest | `id-token: write`, `attestations: write` | Sigstore signing; no write access to contents |
| GitHub Release / Create the release | `contents: write` | Create the release and upload assets |
| Docs / Deploy | `pages: write`, `id-token: write` | GitHub Pages deployment |

Builds, checks, and the audit have read-only access. Third-party actions are
pinned to full commit SHAs of release tags; Dependabot proposes updates.

**Changing owners.** To add or remove a maintainer, a current crates.io
owner runs `cargo owner --add <user>` or `cargo owner --remove <user>`, and a
repository admin edits the reviewers of the `crates-io` environment
(**Settings → Environments → crates-io**). Keep at least two crates.io owners
once a second maintainer exists, so a lost account cannot lock the crate.

**If something leaks or an account is compromised:**

1. Disable publishing: remove the trusted publisher in the crate's
   crates.io settings, and remove or restrict the `crates-io` environment.
2. Revoke every crates.io API token of the affected accounts
   (<https://crates.io/settings/tokens>). Manual publishes used personal
   tokens kept outside the repository; none are needed now.
3. Rotate the account credentials of the affected maintainer (GitHub and
   crates.io passwords, two-factor methods, passkeys, and GitHub personal
   access tokens or SSH keys), and review recent workflow runs and the
   audit log for unexpected dispatches or approvals.
4. If a published version is affected, yank it (`cargo yank --version <v>`),
   delete its GitHub Release assets, and publish a corrected version.
   Published versions can never be replaced.
5. There is no long-lived signing key to rotate: attestations come from
   short-lived Sigstore certificates. After a compromise, treat attestations
   created during the exposure window as untrustworthy and re-verify
   archives against a trusted rebuild with [Reproduce an archive](#reproduce-an-archive).
6. Re-register the trusted publisher only after the cause is fixed.

## Distribution scope

The pipeline above covers reproducible standalone archives for five
targets, checksums, signed provenance, a protected approval, generated
release notes, and documented ownership. Installers and install-script
verification are separate work.

References: [Cargo publishing](https://doc.rust-lang.org/cargo/reference/publishing.html),
[manifest packaging/publication controls](https://doc.rust-lang.org/cargo/reference/manifest.html).
