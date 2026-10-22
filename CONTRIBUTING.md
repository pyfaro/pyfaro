# Contributing to Pyfaro

Thank you for helping build Pyfaro. Please follow our
[code of conduct](CODE_OF_CONDUCT.md).

## Development tools

- Git and Rust through rustup. `rust-toolchain.toml` pins Rust 1.90.0 with
  rustfmt and Clippy; the package's minimum supported Rust version is 1.90.
- Python 3.10+ and uv **0.9.26** for workflow integration tests.
  CI pins Python 3.12 and uv 0.9.26 on all three platforms. Linux, macOS,
  and Windows run creation, dependency, sync, run, Python-version, tool,
  and doctor tests, plus builds, previews, and portable process tests.
  `init --apply` remains Linux-only; other platforms test its explicit
  rejection. Failure scenarios use a portable Rust executable fixture.
- Authenticated GitHub CLI for backlog maintenance.
- Ruff, ty, and pytest are not needed to compile or run the default tests;
  quality-command tests use a portable fixture executable. The real pinned
  tools (Ruff 0.16.10, ty 0.0.85, pytest 9.1.1, the tested versions) are
  exercised by the [end-to-end suite](#end-to-end-tests), which needs network
  access for the first sync. Pyfaro does not install these tools automatically.

```sh
git clone https://github.com/pyfaro/pyfaro.git
cd pyfaro
cargo build --locked
cargo run --locked -- --help
cargo fmt --all -- --check
cargo clippy --locked --all-targets --all-features -- -D warnings
cargo test --locked --all-targets
cargo build --locked --release
```

Run `cargo fmt --all` to apply formatting. CI checks Linux, macOS, and Windows
using the same pinned toolchain. Commit `Cargo.lock` for reproducible builds;
update it intentionally with dependency changes. The weekly security workflow
audits the lockfile against RustSec. To reproduce, install the cargo-audit
version pinned in `.github/workflows/security.yml`, then run `cargo audit`.

## Rust conventions

- Prefer simple, explicit code and one responsibility per module. Keep lines
  within 80 columns where practical. Use rustfmt and heed Clippy warnings.
- Keep `main` thin. `cli` owns argument parsing; future `config`, `core`, and
  `adapters` modules should be introduced only with real implementations.
- Use `Result` for recoverable failures and preserve error sources. Add context
  once at the application boundary. Avoid panics and `unwrap` in runtime code.
- Use `Path`/`PathBuf` and `OsStr`/`OsString` for OS data; do not assume UTF-8,
  concatenate path strings, or change the process-wide working directory.
- Invoke tools with explicit executable and argument arrays, never an implicit
  shell. Stream output, preserve status codes, and design Ctrl-C forwarding
  and child cleanup for Windows and Unix before adding process adapters.
- Do not silently download tools or change a user's environment. Pin tested
  tool versions in fixtures; document supported ranges with adapter changes.
- Preserve standard Python tool configuration and `uv.lock`. New resolution,
  packaging, lockfiles, and replacement quality engines are out of scope.
- Add tests for externally observable behavior and meaningful failure cases.
  Use disposable fixtures and isolated environments for future Python tests.

The CLI uses Clap's help/version status 0 and usage-error status 2. Initialization
errors use status 1; delegated uv failures preserve their numeric exit code;
cancellation uses 130. Test both preservation and idempotence when changing
initialization. See the [init guide](docs/init.md), [create guide](docs/create.md),
[dependency guide](docs/dependencies.md), [sync guide](docs/sync.md),
[run guide](docs/run.md), [Python guide](docs/python.md), and
[doctor guide](docs/doctor.md) for command contracts.
The [uv adapter guide](docs/uv-adapter.md) records delegation and platform
behavior for new workflows.

## End-to-end tests

End-to-end (e2e) tests run the real built `pyfaro` binary against real uv
0.9.26, Ruff 0.16.10, ty 0.0.85 and pytest 9.1.1 in disposable directories
(whose paths contain a space and a non-ASCII character). They assert exit
codes, output and file-system effects, and use fakes only to simulate a tool
that is missing or broken.

Run the suite locally with uv 0.9.26 and Python 3.10 or newer on PATH:

```sh
PYFARO_E2E=1 cargo test --locked --test e2e
```

Without `PYFARO_E2E=1` every e2e test prints a skip note and passes. Network
access is needed the first time tools are fetched into the uv cache. The suite
lives in `tests/e2e/`: `main.rs` declares one module per command group
(`tests/e2e/<group>.rs`), and `harness.rs` provides the shared `Sandbox`,
command builder, `Outcome` assertions, project helpers and file-tree snapshot.
Begin each test with `e2e_guard!();`.

Every new command, and every new behavior of an existing command, must ship
with its e2e test in the same PR. The `E2E (ubuntu-latest)`, `E2E (macos-latest)`
and `E2E (windows-latest)` jobs are required checks; nothing merges until they
pass.

## Branches and pull requests

Start from current `main`; use a focused branch such as `feat/config-loader`,
`fix/project-root`, or `docs/quickstart`. Keep PRs small and explain the reason
for a change, the relevant backlog issue, and verification evidence. Use clear,
imperative commit messages. A PR is expected for changes to protected `main`;
passing CI is required, with no mandatory external approval for a sole
maintainer. Resolve conversations before merging.

## Issues and milestones

Search existing issues before opening one. Use the templates and provide
reproduction steps or a clear proposed outcome. Maintainers triage each issue
with one `type:*`, one `area:*`, and one `priority:*` label:

- `priority:P0`: essential or blocking for the assigned milestone.
- `priority:P1`: important for the assigned milestone.
- `priority:P2`: desirable, deferrable work.

Milestone titles and imported issue scope come from the authoritative manifest
in `tools/github-backlog/`. Preserve symbolic backlog IDs and dependency links.
Do not close an issue merely because a scaffold exists; review every acceptance
criterion. Roadmap versions are not Git tags and do not imply shipped releases.
See the [backlog maintenance guide](tools/github-backlog/README.md).

## Releases

Experimental `0.1.0-alpha.1`, `0.1.0-alpha.2`, and `0.2.0-alpha.1` crates
were published manually. `0.3.0-alpha.1` was the first release made through
the manual Release workflow, which publishes through crates.io trusted
publishing, tags the commit, and creates a GitHub Release with standalone
archives. Future versions use the same workflow. Backlog issue B06 still
owns signing, provenance, and installers for the public beta. The
`v0.1.0-alpha.2`, `v0.2.0-alpha.1`, and `v0.3.0-alpha.1` tags point to their
published source commits and all have GitHub prereleases. To cut a release, follow [How to release](docs/releasing.md#how-to-release).
See [the publishing guide](docs/releasing.md) for verified release records and
explicit approval gates for future uploads, tags, and releases. No registry
credentials are stored in this repository.
