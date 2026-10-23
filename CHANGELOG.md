# Changelog

## Unreleased — 0.4.0-alpha.1 development

Version `0.4.0-alpha.1` has not been published.

- Add `pyfaro create fastapi`, a FastAPI project preset with `--path`,
  `--python`, and `--package`: a typed application with `GET /` and
  `GET /health`, `TestClient` tests, and `fastapi`, `uvicorn[standard]`, and
  a pinned test HTTP client (`httpx2`, or `httpx` on Python 3.10) in the
  `dev` group, validated and rolled back like the other
  templates.
- Add named tasks. `[tool.pyfaro.tasks]` defines argument-vector tasks (an
  array, or a table with `command`, `cwd`, `env` and `description`) and
  explicit shell tasks (`shell`, run with `sh -c` or `cmd.exe /d /s /c`).
  `pyfaro run <task> [-- extra...]` runs one in the project environment with
  the same exit-code and cancellation behavior as `pyfaro run -- ...`.
  A name before `--` is always a task, a program always follows `--`, and
  a program spelled like a task prints a note. Invalid tasks fail before
  launch with the dotted key. The invocation model is recorded in ADR 0002.
- Add `pyfaro dev`, which runs the project's development process inside the
  project environment and supervises it: child output streams through
  unmodified, Pyfaro's own lines start with `pyfaro dev: ` on standard error,
  `--verbose` prints the uv command line, Ctrl-C stops the whole process tree
  (exit 130, `pyfaro dev: stopped`) after a longer graceful shutdown window on
  Unix, and a failing process's exit status is forwarded.
- Add the `[tool.pyfaro.dev]` table (`preset`, `entry-point`, `args`,
  `reload`), validated before launch. The `fastapi` preset runs Uvicorn with
  its own reloader after checking that `uvicorn` is declared and that the port
  is free; without a preset the entry point is run with `python -m`.

## 0.3.0-alpha.1 — 2026-10-22

Published on [crates.io](https://crates.io/crates/pyfaro/0.3.0-alpha.1)
through crates.io trusted publishing by the manual Release workflow
([run 39757664060](https://github.com/pyfaro/pyfaro/actions/runs/39757664060)),
at `2026-10-22T09:50:14.928095Z` from commit `2f041243b9d74cefaa526f017ef84118f95e50a0`.
The workflow created the annotated [`v0.3.0-alpha.1` tag](https://github.com/pyfaro/pyfaro/tree/v0.3.0-alpha.1).
The [GitHub prerelease](https://github.com/pyfaro/pyfaro/releases/tag/v0.3.0-alpha.1)
followed on `2026-10-22T09:51:48Z` with Linux, macOS, and Windows archives,
the `.crate`, and `SHA256SUMS`. The archive SHA-256 is
`67f43a670e90536a00ed55be57524ce094baba837955408ea71e6163ee6922fe`.
Smoke checks with the downloaded Linux binary and uv 0.9.26 covered
`create lib`, `sync`, `check`, and `test`.
This experimental release adds these code-quality capabilities to
`0.2.0-alpha.1`:

- Add `pyfaro lint` and `pyfaro fmt` through Ruff, with project selection,
  paths resolved from the working directory, and `fmt --check` for a
  non-writing format check.
- Add `pyfaro type` through ty, running in the project environment without
  an implicit sync.
- Add `pyfaro test` through pytest in the project environment, forwarding
  arguments and pytest's exit codes unchanged.
- Add `pyfaro check`, which runs the format check, lint, and type check, prints
  a summary, and exits 1 when any step fails. An explicit `--fix` runs
  `ruff check --fix`, then `ruff format`, then repeats the verification.
- Pin pytest 9.1.1, Ruff 0.16.10, and ty 0.0.85 in the `dev` dependency group
  of `create lib` and `create cli` templates.
- Test against Ruff 0.16.10, ty 0.0.85, and pytest 9.1.1. Other versions are
  not rejected, and no quality command syncs the environment implicitly.
- Add manual Release and GitHub Release workflows that attach downloadable
  standalone archives, the `.crate`, and `SHA256SUMS`.

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
