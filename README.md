# Pyfaro

The unified Python development toolchain. Fast, opinionated, and built with
Rust. Inspired by Vite+, Pyfaro aims to bring project setup, dependencies,
quality checks, tests, and development workflows into one cohesive CLI.

## Status

**Latest published alpha: [`0.1.0-alpha.2`](https://crates.io/crates/pyfaro/0.1.0-alpha.2),
published on 2026-10-19.** It implements `init`, `sync`, `run`, and `tools`.
The repository develops **unpublished `0.2.0-alpha.1`** with the v0.2.0
project-management workflows. Interfaces may change between alpha versions.
Completing a backlog milestone does not publish a stable or prerelease crate.
The [GitHub prerelease](https://github.com/pyfaro/pyfaro/releases/tag/v0.1.0-alpha.2)
tags the published source; it does not include standalone binaries.

The current alpha series targets **Linux** for `init --apply`. macOS and Windows
retain builds, help/version, and non-writing previews; applying initialization
there is explicitly unavailable until lifecycle guarantees are implemented.
The other implemented workflows are tested on all three CI platforms.

## Install the alpha

With Rust 1.90 or newer and Cargo installed:

```sh
cargo install pyfaro --version 0.1.0-alpha.2 --locked
pyfaro --version
```

The expected output is `pyfaro 0.1.0-alpha.2`. uv and Python are separate
runtime requirements for applying initialization on Linux and using `sync`
or `run` on supported platforms.

## Try the initialization workflow

Install uv separately and have a suitable Python interpreter available.
The tested uv version is **0.9.26**; older versions are rejected. Pyfaro never
installs uv or downloads Python automatically. The development-only
`pyfaro python install` explicitly allows uv to download an interpreter.

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

`sync` and `run` load the selected project's `pyproject.toml` before invoking
uv. The optional `[tool.pyfaro]` table defaults to schema version `1`; invalid
or unknown Pyfaro settings fail with a path-aware diagnostic. Ruff, ty, uv,
and other tools keep ownership of their own sections. See the
[configuration guide](docs/config.md).

Use `--offline` to disable network access or `--uv <path>` to select an
executable. See the [init guide](docs/init.md) for exact behavior, error
handling, and alpha limitations, and the [release instructions](docs/releasing.md)
for the publication approval gate.

In the published `alpha.2`, `pyfaro sync [PATH]` creates or updates the
project's `.venv` and `uv.lock` through uv. Use `--locked` to reject stale
locks or `--frozen` to install from an existing lock without checking it.
From a nested directory, Pyfaro selects the nearest `pyproject.toml`;
positional `PATH` starts discovery elsewhere, while `--project <PATH>`
selects an exact project instead.
See the [sync guide](docs/sync.md) for the mode differences.

In the unpublished development version, `pyfaro add "requests>=2"` and
`pyfaro remove requests` delegate dependency and lockfile edits to uv.
Use `--dev` or `--group <NAME>` for dependency groups, then run `pyfaro sync`
to update the environment. These commands are not in the published `alpha.2`.
See the [dependency guide](docs/dependencies.md).

`pyfaro run -- <program> [args...]` executes through uv in the project
environment. For example, `pyfaro run -- python -m pytest` runs pytest from
the project. Use `--project <PATH>` to select another project, or `--locked`
to reject a stale lock before execution. Without `--project`, `run` also
discovers the nearest project from a nested directory. See the
[run guide](docs/run.md).

`pyfaro tools` reports the resolved executable path and version for uv, Ruff,
ty, and pytest. Use `pyfaro tools ruff` (or another tool name) to require one
tool and get an actionable error if it is missing or incompatible. Project
tools are found in the selected project's `.venv`, then `VIRTUAL_ENV`, then
`PATH`; uv uses `PATH` or an explicit `--uv <path>`. See the
[tools guide](docs/tools.md).

The unpublished development version adds `pyfaro create lib NAME` and
`pyfaro create cli NAME`. Both generate a packaged `src/` layout with a smoke
test; the CLI variant also creates a console entry point. Creation itself
does not invoke uv or download Python. See the [create guide](docs/create.md)
for naming, interpreter, and build details.

The development version also adds `pyfaro python list`, `python install`, and
`python pin` as thin uv wrappers. Listing and pinning do not download Python;
an explicit `install` may do so unless `--offline` is set. See the
[Python guide](docs/python.md) for project selection and pin behavior.

`pyfaro doctor` is also available in the development version. It reports
project configuration, interpreter and tool versions, layout conflicts, and
missing runtime or default-group distributions without intentionally changing
project files.
Its probes use temporary files and may trigger interpreter or tool startup
side effects. Blocking findings return a non-zero exit code. See the
[doctor guide](docs/doctor.md) for scope, diagnostics, and fixes.

## Planned experience

- Create and initialize Python projects with opinionated, editable defaults.
- Add, remove, and synchronize dependencies through uv.
- Run programs and development servers in the project environment.
- Lint and format with Ruff, type-check with ty, and test with pytest.
- Combine non-destructive quality checks with clear diagnostics.
- Build Python distributions through uv; later add workspaces and task graphs.

Remaining planned commands: `dev`, `lint`, `fmt`, `type`, `check`, `test`, and
`build`.
These commands are roadmap items, not working functionality. The published
`alpha.2` includes `init`, `sync`, `run`, and `tools`; the development version
also includes `create`, `add`, `remove`, `python`, and `doctor`.

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
Running `init --apply`, `sync`, `run`, dependency, Python-version, and doctor
integration tests requires uv and Python. Local template creation does not.
See [CONTRIBUTING.md](CONTRIBUTING.md) for checks and development conventions.

## Roadmap

Milestones are planning targets, not release announcements or dates.

| Milestone | Focus |
| --- | --- |
| v0.1.0 — Foundation | Architecture, CLI, configuration, process runner, CI |
| v0.2.0 — Project Management | Creation, environments, dependencies, Python versions, run, doctor |
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
