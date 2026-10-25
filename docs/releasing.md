# Publishing Pyfaro

The latest published version is [`0.3.0-alpha.1`](https://crates.io/crates/pyfaro/0.3.0-alpha.1).
It adds `lint`, `fmt`, `type`, `test`, and `check` to the project creation,
dependency, Python-version, and doctor commands of `0.2.0-alpha.1`.
`init --apply` remains Linux-only; the other commands are tested on Linux,
macOS, and Windows. `0.4.0-alpha.1` (v0.4.0 Developer Experience work) is prepared and awaits
publication. The repository now develops unpublished `0.5.0-beta.1`
(v0.5.0 Public Beta work). Its version number is not a publication
approval.

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
2. **Dry run (optional).** Package and test without uploading:

   ```sh
   gh workflow run release.yml -f version=<version>
   gh run watch "$(gh run list --workflow release.yml --limit 1 --json databaseId --jq '.[0].databaseId')"
   ```

   Download the `crate` artifact from the run to review the archive and its
   SHA-256.
3. **Publish.** Start the real run from `main`:

   ```sh
   gh workflow run release.yml -f version=<version> -f publish=true
   ```

   Preflight, CI on all three platforms, the audit, and packaging run first.
4. **Approve.** The **Publish to crates.io** job waits on the `crates-io`
   environment. Approve it in the run page (**Review deployments**), or:

   ```sh
   run=<run id>
   env_id=$(gh api "repos/pyfaro/pyfaro/actions/runs/$run/pending_deployments" --jq '.[0].environment.id')
   gh api -X POST "repos/pyfaro/pyfaro/actions/runs/$run/pending_deployments" \
     -F "environment_ids[]=$env_id" -f state=approved -f comment="Approve <version>"
   ```

   The workflow then uploads, tags `v<version>`, builds the release archives,
   creates the GitHub Release, and reinstalls the version from crates.io.
5. **Verify.** Check the crates.io page, the tag, and the release assets:

   ```sh
   gh release download v<version> -p 'SHA256SUMS' -p '*x86_64-unknown-linux-gnu.tar.gz'
   sha256sum -c --ignore-missing SHA256SUMS
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
3. **Package** builds the archive, runs all-target tests from the unpacked
   package, runs `cargo publish --dry-run`, and keeps the `.crate` and its
   SHA-256 as the `crate` workflow artifact for review.
4. **Publish** runs only with `publish` checked. It waits for approval on the
   `crates-io` environment, obtains a short-lived token through crates.io
   trusted publishing, uploads, and fails if the registry checksum differs
   from the reviewed archive. It then creates the annotated `v<version>` tag
   on the exact workflow commit.
5. **GitHub Release** runs unless `github-release` is unchecked. It calls
   `.github/workflows/github-release.yml`, described below.
6. **Install from crates.io** installs the published version with `--locked`
   and checks its `--version` output.

Run it once with `publish` unchecked to obtain the archive and checksum for
review, then run it again with `publish` checked after approval. Afterward,
record the release in a follow-up PR as for earlier versions and move the
development version forward.

One-time setup, done by a crates.io owner and a repository admin:

- On crates.io, add a trusted publisher for `pyfaro`: repository
  `pyfaro/pyfaro`, workflow `release.yml`, environment `crates-io`.
  No long-lived registry token is stored in GitHub.
- In GitHub, keep the `crates-io` environment restricted to `main` with the
  maintainer as a required reviewer.

If the upload succeeds but tagging fails, do not rerun the workflow:
preflight rejects the published version. Create the missing tag on the
workflow's commit, following the approval steps above, then run the GitHub
Release workflow for it.

### GitHub Release workflow

`.github/workflows/github-release.yml` builds the downloadable assets for a
version that is already on crates.io and tagged, and creates its GitHub
Release. The Release workflow calls it after publishing. Run it by hand to
backfill an older tag or refresh its assets:

```sh
gh workflow run github-release.yml -f tag=v0.2.0-alpha.1
```

It checks that the tag exists and the version is on crates.io, then builds
the tagged source with the pinned toolchain for:

| Target | Runner | Archive |
| --- | --- | --- |
| `x86_64-unknown-linux-gnu` | Ubuntu 22.04 | `.tar.gz` |
| `aarch64-unknown-linux-gnu` | Ubuntu 22.04 Arm | `.tar.gz` |
| `aarch64-apple-darwin` | macOS | `.tar.gz` |
| `x86_64-apple-darwin` | macOS, cross-compiled | `.tar.gz` |
| `x86_64-pc-windows-msvc` | Windows | `.zip` |

Each archive holds the `pyfaro` executable, `README.md`, `LICENSE`, and
`CHANGELOG.md`. Native builds must report the exact version; the
cross-compiled macOS build is not executed. The release also attaches the
`.crate` downloaded from crates.io, after checking it against the registry
checksum, and a `SHA256SUMS` file for every asset.

The notes are the `## <version>` section of `CHANGELOG.md` from the
triggering ref (the released commit, or `main` for a backfill), followed by
install instructions and GitHub's generated list of merged pull requests
since the previous release. Versions with a pre-release suffix become
prereleases. When the release already exists, its notes are rebuilt the
same way, including a regenerated pull request list, and its assets are
re-uploaded. The binaries are unsigned and have no
provenance attestation yet.

## Public beta distribution remains B02/B06

The alpha workflows above already provide trusted publishing, a protected
release approval, checksums, generated release notes, and standalone
archives. The beta work retains responsibility for the supported-platform
policy, signed binaries and artifact provenance, installers, and documented
credential rotation.

References: [Cargo publishing](https://doc.rust-lang.org/cargo/reference/publishing.html),
[manifest packaging/publication controls](https://doc.rust-lang.org/cargo/reference/manifest.html).
