# Changelog

## Unreleased — 0.1.0-alpha.2 development

- Extract a shared, version-gated uv adapter from `init`, preserving OS-native
  arguments and delegated output and exit codes.
- Add process-group and Windows Job Object containment for future delegated
  workflows. `init --apply` remains Linux-only in this alpha series.
- Add `pyfaro sync [PATH]` with uv-owned environment/lockfile updates,
  `--locked` stale-lock rejection, and `--frozen` lockfile reuse.
- Add `pyfaro run -- <program> [args...]` with OS-native passthrough, project
  environment selection, terminal-input relay, and delegated exit status.

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
