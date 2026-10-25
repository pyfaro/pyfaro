# Changelog

## 0.5.0-beta.1 — 2026-10-25

The first public beta. It adds these build, distribution and diagnostics
capabilities to `0.4.0-alpha.1`:

- Publish the CLI and configuration reference. The command-line reference now
  documents every command and subcommand with its usage line, every argument
  and flag with the help text Pyfaro prints, and its exit codes. A new
  configuration reference lists every `[tool.pyfaro]` key with its type,
  default, rule and error message. The installation, troubleshooting (adds
  `pyfaro build` failures and tab completion) and environment-variable pages
  are brought up to date. Unit tests keep the pages from drifting: they compare
  the reference with the real argument definitions and `--help` output, and
  validate every `[tool.pyfaro]` TOML example in the documentation with
  Pyfaro's own configuration loader.
- Add `pyfaro build`, which builds a source distribution and a wheel through
  `uv build` with the project's own `[build-system]` backend (`uv_build`,
  hatchling, setuptools or none). `--sdist` and `--wheel` select a kind,
  `--out-dir` chooses the directory (default `dist`), `--clear` empties it
  first (refused when it would delete the project), and uv's exit status is
  forwarded. Pyfaro lists the artifacts each run created or rewrote as
  `built <path>` lines and fails when an expected kind is missing.
- Document the privacy and trust model: a new "Security and privacy" page
  (site and `docs/security-and-privacy.md`) states that Pyfaro itself makes no
  network connections and sends no telemetry, lists per command which uv
  operations can reach the network and how `--offline`, `UV_OFFLINE` and
  `UV_PYTHON_DOWNLOADS` affect them, names the third-party programs Pyfaro
  runs and where they come from, and describes updates and release trust.
  `SECURITY.md` links it and now supports the latest published version only,
  with fixes on `main`. A new test (`tests/privacy.rs`) fails when an HTTP
  or TLS client, telemetry or self-update crate enters `Cargo.lock`.
- Add a global `-v`/`--verbose` flag, accepted before the subcommand for every
  command and after it for every command except `test` (whose arguments after
  `test`, `-v` and `--verbose` included, still go to pytest verbatim; use
  `pyfaro -v test` for Pyfaro's trace). Pyfaro then prints to standard error,
  before launching anything, each tool it resolved with its version and path
  and every command line it executes, in the `pyfaro <command>: <message>`
  format `pyfaro dev` already used. `pyfaro dev --verbose` keeps working; it
  now also lists the uv version probe and the uv version, so the `running`
  line for the launch is no longer the first one. Output without the flag is
  unchanged.
- Mask credentials: the password of a URL's userinfo prints as `****`
  (`https://user:****@host`, and `https://****@host` for a lone token) in
  verbose lines, error messages, Clap usage errors, the `doctor` report and
  text taken from tool diagnostics, and environment variable values are never
  printed (only the names of a task's `env`). Output that child processes
  write themselves is not rewritten.
- Document the exit codes as a stable contract for the public beta
  (`docs/exit-codes.md` and the site's exit-codes page): `0`, `1`
  (Pyfaro failures, a failing `check`, `doctor` errors, a tool that ends
  without a numeric status), `2` (usage), forwarded tool codes and `130`.
- Make the release pipeline build and attest its artifacts. The Release
  workflow now builds the five standalone archives (Linux x86_64 and
  aarch64, macOS x86_64 and aarch64, Windows x86_64) before the crates.io
  upload, so a broken target blocks it, and builds each twice on separate
  runners, failing unless the binaries and archives are byte-for-byte
  identical (`--remap-path-prefix`, commit-time timestamps, sorted archives
  with fixed owners and modes). It checks that the crate, tag, and every
  runnable binary's `--version` agree. Every archive and the `.crate` get a
  GitHub artifact attestation (SLSA build provenance, verify with
  `gh attestation verify <file> --repo pyfaro/pyfaro`), and
  `docs/releasing.md` documents how to reproduce an archive, verify a
  download, and who owns the release credentials.
- Add install scripts, served by the documentation site: `install.sh` for
  Linux and macOS (`curl --proto '=https' --tlsv1.2 -LsSf
  https://pyfaro.dev/install.sh | sh`) and `install.ps1` for Windows
  PowerShell 5.1 and 7. They detect the platform, find the newest release
  (prereleases included) or `PYFARO_VERSION`, download the archive and
  `SHA256SUMS` over HTTPS only, verify the SHA-256 and stop without installing
  anything if the checksum is missing, wrong or cannot be computed, then
  install to `PYFARO_INSTALL_DIR` or `~/.local/bin`. They never use `sudo`,
  never edit shell profiles or `PATH`, and never install uv; running them
  again upgrades in place. `PYFARO_DOWNLOAD_BASE` (`https://` or a local
  `file://` mirror) exists for testing. CI installs the published releases on
  Linux, macOS and Windows and checks that a corrupted download is refused.
- Rework the installation guide: the script (with a "download, inspect, then
  run" variant), Cargo and archives, with upgrade and uninstall for each
  method on every supported platform.
- `pyfaro doctor` and every command that needs uv now explain a missing, too
  old or broken uv in one line with the next step: the install link and
  command, `uv self update` or a reinstall, or `--uv <path>`. This replaces
  the chained `could not inspect uv at uv: could not find uv at uv: cannot
  find binary path; ...` message. Exit codes are unchanged.
- Add `pyfaro completions <SHELL>`, which prints a static tab-completion script
  for `bash`, `zsh`, `fish` or `powershell`. It needs no project and no uv. The
  help of `pyfaro --help` now shows a short workflow and a link to
  https://pyfaro.dev/ instead of a stale status line, and `create`, `sync`,
  `add`, `run`, `dev`, `test`, `check`, `python` and `completions` end with
  examples that are checked against the argument definitions. Argument help
  now uses the same wording for `--project`, `--uv`, `--offline`, `--locked`
  and `--frozen` on every command; no flag, default or behavior changed.
- Publish Pyfaro to PyPI as binary wheels, so `uv tool install pyfaro`,
  `pipx install pyfaro` and `pip install pyfaro` work without a Rust
  toolchain. The Release workflow builds, with maturin (`bindings = "bin"`,
  metadata in `python/pyproject.toml`), one wheel holding only the `pyfaro`
  executable for each release target (`manylinux_2_17` on Linux), plus an
  sdist, before the crates.io upload. It installs each native wheel in a fresh
  virtual environment and checks `pyfaro --version`, attests the files like
  the archives, attaches them to the GitHub Release and `SHA256SUMS`, and
  uploads them to PyPI from the gated `pypi` environment through trusted
  publishing, with no token. While only prereleases exist, install with
  `uv tool install --prerelease allow pyfaro` or an exact version such as
  `pyfaro==0.5.0b1`.

## 0.4.0-alpha.1 — 2026-10-23

Published on [crates.io](https://crates.io/crates/pyfaro/0.4.0-alpha.1)
through crates.io trusted publishing by the manual Release workflow
([run 39950335350](https://github.com/pyfaro/pyfaro/actions/runs/39950335350)),
at `2026-10-25T10:10:09.854629Z` from commit `107704e8e4e4f7c728f905e77e7e5ce5531c8880`.
The workflow created the annotated [`v0.4.0-alpha.1` tag](https://github.com/pyfaro/pyfaro/tree/v0.4.0-alpha.1).
The [GitHub prerelease](https://github.com/pyfaro/pyfaro/releases/tag/v0.4.0-alpha.1)
followed on `2026-10-25T10:11:48Z` with Linux, macOS, and Windows archives,
the `.crate`, and `SHA256SUMS`. The archive SHA-256 is
`3280c65c0bb99d1af90e79bb7c7e325958400cd5e227f3bd41cbdf40484ba74d`.
Smoke checks with the downloaded Linux binary and uv 0.9.26 covered
`create fastapi`, `sync`, `check`, and `test`.
This experimental release adds these developer-experience capabilities to
`0.3.0-alpha.1`:

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
- Keep `pyfaro dev` running across Uvicorn reloads on Windows. The console
  delivers Uvicorn's reload Ctrl-C to every attached process, so `pyfaro dev`
  now waits about five seconds for the tree to end by itself before treating
  it as a stop (`pyfaro dev: stopped`, exit 130), carries on when it does not,
  and stops the tree at once on a second Ctrl-C. Other commands are unchanged.
- Make Uvicorn reloads under `pyfaro dev` reliable on Windows: a console only
  delivers the Ctrl-C that Uvicorn's reloader generates once something uses
  it, so on a console reached only through pipes the worker never restarted.
  `pyfaro dev` now touches the console while it supervises.
- Wire `pyfaro dev` into the FastAPI template: `pyfaro create fastapi` now
  writes `[tool.pyfaro.dev]` with `preset = "fastapi"` and the
  `<package>.main:app` entry point (following `--package`), and the generated
  README serves the app with `pyfaro dev` instead of a hand-written Uvicorn
  command.
- Add the FastAPI getting-started guide, from install to a running app and a
  passing CI workflow, and update the README and the site so `dev`, named
  tasks and the FastAPI preset are no longer described as planned.

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
