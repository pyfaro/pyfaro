# Publishing Pyfaro

The repository prepares `0.1.0-alpha.2` as an unpublished release candidate.
The published version is still `0.1.0-alpha.1`. This candidate adds `sync`,
`run`, and `tools` to the experimental `init` workflow. `init --apply` remains
Linux-only; `sync` and `run` are tested on Linux, macOS, and Windows.

Completing the `v0.1.0 — Foundation` backlog milestone means its source-work
criteria are met, not that a stable `0.1.0` crate has shipped. The current
package version is already `0.1.0-alpha.2`; this preparation verifies it
without uploading. CI package and publish dry-runs do not upload the crate.
Before publication, the maintainer must explicitly approve this exact version
and the merged commit after reviewing the candidate archive and CI evidence.
Approval of the preparation PR alone is not publication approval.

## Candidate: 0.1.0-alpha.2

The source candidate is tracked by [release-preparation issue #77](https://github.com/pyfaro/pyfaro/issues/77).
The reviewed PR and its merge commit identify the candidate source. Record
the package checksum and verification evidence with that issue after merge;
do not put a candidate checksum in this packaged document, since changing it
would change the archive. Recheck crates.io immediately before an upload to
confirm that this version is still unused. A separate approval is required
before any upload, tag, or GitHub Release.

## First manual alpha: 0.1.0-alpha.1

The first crates.io alpha was published on **2026-10-19**, ahead of the public-beta
milestone. It contains the useful but experimental [`init`](init.md) workflow.
This does not move roadmap milestones or expand B06: automated distribution,
standalone artifacts, and GitHub Releases still belong to B02/B06 for beta.
The first alpha supports `init --apply` on Linux only, as explicitly agreed
for the early release. macOS and Windows retain compilation and previews;
their application workflow is deferred pending process-lifecycle guarantees.

Publication stayed disabled during bootstrap. With the initialization slice
implemented and tested, the release-preparation PR restricts publication to
`publish = ["crates-io"]`. This permits verification and manual uploads;
each upload still requires explicit authorization. No workflow uploads this crate.

**Do not run an uploading `cargo publish` until the maintainer explicitly
approves this exact release and commit.** Merging the preparation PR, passing
CI, or accepting a dry-run is not that approval.

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

This version already exists and must not be published again. The checklist
below applies to the `alpha.2` candidate. No release tag or GitHub Release was
created for `alpha.1`.

### Name and ownership

On 2026-10-15, the official crates.io API returned HTTP 404 with
`crate pyfaro does not exist` for:

<https://crates.io/api/v1/crates/pyfaro>

The name was unregistered at that check, not reserved. The first publication
subsequently registered it under `mhbxyz`. Confirm the publishing account and
owner permissions before future uploads; credentials remain local and must
not be committed to this repository.

## Pre-publication verification for alpha.2

Use a clean checkout of the approved preparation commit and the pinned
Rust toolchain. Install uv 0.9.26 separately and provide Python 3.10+ (CI uses
3.12). No publishing token is needed for these verification commands:

```sh
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
and initialization/release/architecture docs are explicitly included.
Development graph/cache state, GitHub/backlog automation, and build outputs
must not enter the archive. Cargo also adds its normalized/original manifests
and VCS information. Inspect `target/package/pyfaro-0.1.0-alpha.2.crate`
and record its SHA-256 checksum outside the package.

Smoke-test a dependency-free new project and an existing project with uv:

```sh
cargo run --locked -- init ../pyfaro-alpha-smoke
cargo run --locked -- init ../pyfaro-alpha-smoke --apply --offline
cargo run --locked -- init ../pyfaro-alpha-smoke --apply --offline
```

Also run `cargo test --locked --all-targets` from the unpacked package to
verify the shipped tests and fixtures are self-contained. Check the
`--version` output is `pyfaro 0.1.0-alpha.2`. Review the candidate changelog
and CI evidence on the preparation PR: full Rust checks on Linux, macOS,
and Windows, plus the RustSec audit. Smoke-test `sync`, `run`, and `tools`
from the candidate binary before requesting publication approval.

## Only after explicit publication approval

1. Confirm the clean commit, name availability, crates.io account, ownership
   plan, successful checks, and version with the maintainer. Authenticate
   locally through Cargo; do not put a token in the repository or logs.
2. Publish the approved commit manually:

   ```sh
   cargo publish --locked
   ```

3. Verify the crates.io entry and test installation in a disposable location:

   ```sh
   cargo install pyfaro --version 0.1.0-alpha.2 --locked --root <temporary-prefix>
   ```

4. Confirm the resulting binary's version and smoke-test `init`, `sync`,
   `run`, and `tools` with the external uv/Python requirements. Record the
   actual publication date and registry checksum in a follow-up reviewed PR;
   update the changelog, release record, and README's publication status.
5. Create/push a release tag only after publication is verified and separately
   authorized. This alpha does not introduce a GitHub Release pipeline.

A published crate version cannot be overwritten. If a defective alpha ships,
agree on yanking it and publishing a corrected alpha with a new version;
never retag or silently reuse a published version. Rotate/revoke credentials
if compromised.

## Public beta distribution remains B02/B06

The existing beta work retains responsibility for a supported-platform
matrix, standalone executable builds, tag-to-release automation, least-
privilege credentials or trusted publishing, protected release approval,
checksums, artifact provenance, generated release notes, and documented
recovery/credential rotation. No alpha preparation closes those tickets.

References: [Cargo publishing](https://doc.rust-lang.org/cargo/reference/publishing.html),
[manifest packaging/publication controls](https://doc.rust-lang.org/cargo/reference/manifest.html).
