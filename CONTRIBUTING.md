# Contributing to Pyfaro

Thank you for helping build Pyfaro. Please follow our
[code of conduct](CODE_OF_CONDUCT.md).

## Development tools

- Git and Rust through rustup. `rust-toolchain.toml` pins Rust 1.90.0 with
  rustfmt and Clippy; the package's minimum supported Rust version is 1.90.
- Python 3.10+ and uv **0.9.26** for workflow integration tests.
  CI pins Python 3.12 and uv 0.9.26 on all three platforms. macOS and
  Windows run `sync`, `run`, and tool-discovery tests, plus builds,
  previews, explicit rejection of `init --apply`, and portable process tests.
  Successful uv scenarios run
  offline in disposable directories without third-party package installation.
  Failure scenarios use a portable Rust executable fixture.
- Authenticated GitHub CLI for backlog maintenance.
- Ruff, ty, and pytest are not needed to compile or run fixture-based discovery
  tests. `pyfaro tools` inspects installed copies; future workflow adapters
  will use them. Pyfaro does not install these tools automatically.

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
initialization. See the [init guide](docs/init.md), [sync guide](docs/sync.md),
[run guide](docs/run.md), and the ADR for the contract.
The [uv adapter guide](docs/uv-adapter.md) records delegation and platform
behavior for new workflows.

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

Publishing is future work tracked by backlog issue B06. See
[the publishing plan](docs/releasing.md). Bootstrap does not publish crates,
create release tags, or configure registry credentials.
