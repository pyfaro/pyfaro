# Publishing Pyfaro

## First manual alpha: 0.1.0-alpha.1

The first crates.io publication is being prepared ahead of the public-beta
milestone. It contains the useful but experimental [`init`](init.md) workflow.
This does not move roadmap milestones or expand B06: automated distribution,
standalone artifacts, and GitHub Releases still belong to B02/B06 for beta.
The first alpha supports `init --apply` on Linux only, as explicitly agreed
for the early release. macOS and Windows retain compilation and previews;
their application workflow is deferred pending process-lifecycle guarantees.

Publication stayed disabled during bootstrap. With the initialization slice
implemented and tested, the release-preparation PR restricts publication to
`publish = ["crates-io"]`. This permits verification and an eventual manual
upload; it does **not** authorize publishing. No workflow uploads this crate.

**Do not run an uploading `cargo publish` until the maintainer explicitly
approves this exact release and commit.** Merging the preparation PR, passing
CI, or accepting a dry-run is not that approval.

### Name and ownership

On 2026-10-15, the official crates.io API returned HTTP 404 with
`crate pyfaro does not exist` for:

<https://crates.io/api/v1/crates/pyfaro>

The name was unregistered at that check, not reserved. Recheck before upload.
Confirm the publishing account/organization and intended owners with the
maintainer. Credentials and ownership have not been configured by this PR.

### Pre-publication verification

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
and VCS information. Inspect `target/package/pyfaro-0.1.0-alpha.1.crate`.

Smoke-test a dependency-free new project and an existing project with uv:

```sh
cargo run --locked -- init ../pyfaro-alpha-smoke
cargo run --locked -- init ../pyfaro-alpha-smoke --apply --offline
cargo run --locked -- init ../pyfaro-alpha-smoke --apply --offline
```

Also run `cargo test --locked --all-targets` from the unpacked package to
verify the shipped tests and fixtures are self-contained. Check the
`--version` output is `pyfaro 0.1.0-alpha.1`. Review the pending changelog
and CI evidence on the preparation PR: full Linux workflow checks plus
macOS/Windows build, preview, and unsupported-apply checks.

### Only after explicit publication approval

1. Confirm the clean commit, name availability, crates.io account, ownership
   plan, successful checks, and version with the maintainer. Authenticate
   locally through Cargo; do not put a token in the repository or logs.
2. Publish the approved commit manually:

   ```sh
   cargo publish --locked
   ```

3. Verify the crates.io entry and test installation in a disposable location:

   ```sh
   cargo install pyfaro --version 0.1.0-alpha.1 --locked --root <temporary-prefix>
   ```

4. Confirm the resulting binary's version and smoke-test `init` with the
   external uv/Python requirements. Record the actual publication date in
   the changelog and update the README's pending-publication status.
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
