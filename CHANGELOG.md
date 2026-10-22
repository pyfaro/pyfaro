# Changelog

## Unreleased — 0.3.0-alpha.1 development

This version has not been published. No changes yet.

## 0.2.0-alpha.1 — 2026-10-22

Published on [crates.io](https://crates.io/crates/pyfaro/0.2.0-alpha.1).
The annotated [`v0.2.0-alpha.1` tag](https://github.com/pyfaro/pyfaro/tree/v0.2.0-alpha.1)
points to the published source. The [GitHub prerelease](https://github.com/pyfaro/pyfaro/releases/tag/v0.2.0-alpha.1)
followed on `2026-10-22T09:05:49Z` with Linux, macOS, and Windows archives,
the published `.crate`, and `SHA256SUMS`.
This experimental release adds these capabilities to `0.1.0-alpha.2`:

- Add `pyfaro add` and `pyfaro remove` for uv-owned dependency and lockfile
  edits, including development and named dependency groups. Environment
  synchronization remains an explicit `pyfaro sync` step.
- Add `pyfaro create lib` and `pyfaro create cli` with packaged source layouts,
  a uv build backend, smoke tests, and independent distribution, package, and
  command names.
- Add uv-backed `pyfaro python list|install|pin`. Only explicit installation
  permits interpreter downloads; pinning follows uv's project constraints.
- Add a read-only `pyfaro doctor` report for project configuration, Python and
  tool versions, layout conflicts, and missing runtime distributions.

The published archive comes from commit
[`c7d185c8c0a247396104b183c8473bddcd299856`](https://github.com/pyfaro/pyfaro/commit/c7d185c8c0a247396104b183c8473bddcd299856)
and was registered at `2026-10-22T08:37:08.585547Z`. Its registry and
locally packaged archive SHA-256 is
`15c76a1da74c8114d0d3a8b3bb16c03a5985251f5703c94c66960bb2ab9bda37`.
Installation from crates.io and Linux `init` preview/apply/reapply, `sync`,
`run`, `tools`, `doctor`, `create lib`, and `create cli` smoke checks passed
with uv 0.9.26. Reapplying left `pyproject.toml` and `uv.lock` byte-for-byte
unchanged.

## 0.1.0-alpha.2 — 2026-10-19

Published on [crates.io](https://crates.io/crates/pyfaro/0.1.0-alpha.2).
The annotated [`v0.1.0-alpha.2` tag](https://github.com/pyfaro/pyfaro/tree/v0.1.0-alpha.2)
points to the published source. A manual
[GitHub prerelease](https://github.com/pyfaro/pyfaro/releases/tag/v0.1.0-alpha.2)
followed on `2026-10-19T06:19:32Z`, without standalone binaries.
This experimental release adds these capabilities to `alpha.1`:

- Load and validate typed Pyfaro configuration from the selected project's
  `pyproject.toml`, with a non-writing schema default and path-aware errors.
- Add `pyfaro tools [uv|ruff|ty|pytest]` to inspect resolved executables and
  versions, with project environment precedence and actionable strict checks.
- Discover the nearest project for `sync` and `run` from nested directories,
  with explicit project selection and deterministic symlink resolution.
- Extract a shared, version-gated uv adapter from `init`, preserving OS-native
  arguments and delegated output and exit codes.
- Add process-group and Windows Job Object containment for future delegated
  workflows. `init --apply` remains Linux-only in this alpha series.
- Add `pyfaro sync [PATH]` with uv-owned environment/lockfile updates,
  `--locked` stale-lock rejection, and `--frozen` lockfile reuse.
- Add `pyfaro run -- <program> [args...]` with OS-native passthrough, project
  environment selection, terminal-input relay, and delegated exit status.

The published archive comes from commit
[`be259107668c1a34dfd52d24ec10fa14a8563cbb`](https://github.com/pyfaro/pyfaro/commit/be259107668c1a34dfd52d24ec10fa14a8563cbb)
and was registered at `2026-10-19T05:58:23.344503Z`. Its registry and
downloaded-archive SHA-256 is
`e52fd92c157227330e3894f55abd7b171317cd3788ec5596f9bd12aabf5fbfd1`.
Installation from crates.io and offline Linux `init` preview/apply/reapply,
`sync`, `run`, and `tools` smoke checks passed with uv 0.9.26. Reapplying
left `pyproject.toml` and `uv.lock` byte-for-byte unchanged.

## 0.1.0-alpha.1 — 2026-10-19

First experimental release, published on
[crates.io](https://crates.io/crates/pyfaro/0.1.0-alpha.1). CLI and
configuration interfaces may change during the alpha series.

### Added

- `pyfaro init [PATH]`: non-writing preview and explicit `--apply` for
  standalone Python projects, using uv for new metadata and `uv.lock`.
- Minimal, validated `[tool.pyfaro]` configuration, preservation of existing
  project/tool settings, idempotent initialization, `--offline`, and `--uv`.
- Restoration of metadata/lockfiles on delegated failures or cancellation,
  process-tree cleanup, and actionable uv availability/version diagnostics.
- Unit tests, real-uv integration tests, portable failure/cancellation
  fixtures, pinned uv CI, and package/publication dry-run checks.
- Minimal Rust CLI with help and version output.
- Architecture and contributor documentation, Apache-2.0 licensing, and
  community and security policies.
- Multi-platform Rust CI, dependency auditing, and Dependabot updates.
- Official planning backlog and resumable GitHub import tooling.

### Release scope

- First manual crates.io alpha preparation is independent of B06. B06
  remains responsible for automated distribution and GitHub Releases for
  the public beta. Existing roadmap milestones have not been moved.
- Only initialization is implemented; workspaces and other workflow
  commands remain planned.
- This first alpha supports applying initialization on Linux. macOS and
  Windows retain builds and previews but explicitly reject `--apply` until
  their process-lifecycle guarantees are implemented.
