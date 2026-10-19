# Pyfaro

The unified Python development toolchain. Fast, opinionated, and built with
Rust. Inspired by Vite+, Pyfaro aims to bring project setup, dependencies,
quality checks, tests, and development workflows into one cohesive CLI.

## Status

**Experimental alpha — [`0.1.0-alpha.1`](https://crates.io/crates/pyfaro/0.1.0-alpha.1),
published on 2026-10-19.** The published alpha implements `init`, `--help`,
and `--version`. Interfaces may change between alpha versions.
The development branch adds `sync` and `run`, and targets an unpublished
`0.1.0-alpha.2` build. The installation command below installs the published
`alpha.1`.

The first alpha targets **Linux** for `init --apply`. macOS and Windows
retain builds, help/version, and non-writing previews; applying initialization
there is explicitly unavailable until lifecycle guarantees are implemented.
The development `sync` and `run` commands operate on all three CI platforms.

## Install the alpha

With Rust 1.90 or newer and Cargo installed:

```sh
cargo install pyfaro --version 0.1.0-alpha.1 --locked
pyfaro --version
```

The expected output is `pyfaro 0.1.0-alpha.1`. uv and Python are separate
runtime requirements for applying initialization on Linux.

## Try the initialization workflow

Install uv separately and have a suitable Python interpreter available.
The tested uv version is **0.9.26**; older versions are rejected. Pyfaro never
installs uv or downloads Python automatically.

Preview initialization of a new or existing standalone project, then
explicitly apply it:

```sh
pyfaro init ./my-project
pyfaro init ./my-project --apply
```

`init` delegates new project metadata and dependency locking to uv, adds
`[tool.pyfaro]` with `schema-version = 1`, and preserves existing dependency
declarations and third-party tool settings. It does not install dependencies
or create a virtual environment. A second run is idempotent for an unchanged
project and dependency resolution.

Use `--offline` to disable network access or `--uv <path>` to select an
executable. See the [init guide](docs/init.md) for exact behavior, error
handling, and alpha limitations, and the [release instructions](docs/releasing.md)
for the publication approval gate.

In the development build, `pyfaro sync [PATH]` creates or updates the
project's `.venv` and `uv.lock` through uv. Use `--locked` to reject stale
locks or `--frozen` to install from an existing lock without checking it.
See the [sync guide](docs/sync.md) for the mode differences.

`pyfaro run -- <program> [args...]` executes through uv in the project
environment. For example, `pyfaro run -- python -m pytest` runs pytest from
the project. Use `--project <PATH>` to select another project, or `--locked`
to reject a stale lock before execution. See the [run guide](docs/run.md).

## Planned experience

- Create and initialize Python projects with opinionated, editable defaults.
- Add, remove, and synchronize dependencies through uv.
- Run programs and development servers in the project environment.
- Lint and format with Ruff, type-check with ty, and test with pytest.
- Combine non-destructive quality checks with clear diagnostics.
- Build Python distributions through uv; later add workspaces and task graphs.

Remaining planned commands: `create`, `add`, `remove`, `dev`,
`lint`, `fmt`, `type`, `check`, `test`, `build`, and `doctor`.
These commands are roadmap items, not working functionality. The initial
`init` slice is available ahead of the broader project-management milestone.

## Integration over reimplementation

| Responsibility | Owner |
| --- | --- |
| CLI, configuration, orchestration, diagnostics, workflows | Pyfaro |
| Python, environments, resolution, dependencies, packaging | uv |
| Linting and formatting | Ruff |
| Static typing | ty; alternate checkers may be considered later |
| Test execution | pytest |

Pyfaro will use `pyproject.toml` and uv's `uv.lock`. It will not introduce
another dependency solver, package manager, Python installer, lockfile format,
linter, type checker, or test engine. The current code keeps each workflow
in a focused module and delegates environment and package operations to uv.
Modules will grow when there is functionality to own.
See [ADR 0001](docs/architecture/0001-project-architecture.md).

## Build from source

Install [Rust through rustup](https://rustup.rs/) and Git. The checked-in
toolchain selects Rust 1.90.0, rustfmt, and Clippy.

```sh
git clone https://github.com/pyfaro/pyfaro.git
cd pyfaro
cargo build --locked
cargo run --locked -- --help
cargo run --locked -- --version
```

Python, uv, Ruff, ty, and pytest are not needed to compile the Rust binary.
Running `init --apply`, `sync`, `run`, and their integration tests requires uv and
Python.
See [CONTRIBUTING.md](CONTRIBUTING.md) for checks and development conventions.

## Roadmap

Milestones are planning targets, not release announcements or dates.

| Milestone | Focus |
| --- | --- |
| v0.1.0 — Foundation | Architecture, CLI, configuration, process runner, CI |
| v0.2.0 — Project Management | Creation, environments, dependencies, run |
| v0.3.0 — Code Quality | Ruff, ty, pytest; first useful MVP |
| v0.4.0 — Developer Experience | Development mode, presets, simple tasks |
| v0.5.0 — Public Beta | Packaging, installers, public distribution |
| v1.0.0 — Stable Release | Compatibility, security, stability |
| v1.1.0 — Workspaces & Tasks | Workspace orchestration, DAG, local cache |
| v1.2.0 — Agent Integration | Structured diagnostics and agent workflows |

Follow the [development roadmap](https://github.com/orgs/pyfaro/projects/1),
[issues](https://github.com/pyfaro/pyfaro/issues), and
[milestones](https://github.com/pyfaro/pyfaro/milestones).
The authoritative [backlog manifest](tools/github-backlog/pyfaro-issues.json)
is included alongside its import tools. Tickets close only after their full
acceptance criteria have been reviewed.

## Community and security

Contributions are welcome. Read our [contribution guide](CONTRIBUTING.md),
[code of conduct](CODE_OF_CONDUCT.md), and [security policy](SECURITY.md).
General questions belong in
[Discussions](https://github.com/pyfaro/pyfaro/discussions).

## License

Pyfaro is licensed under [Apache-2.0](LICENSE).
