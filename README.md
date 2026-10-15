# Pyfaro

The unified Python development toolchain. Fast, opinionated, and built with
Rust. Inspired by Vite+, Pyfaro aims to bring project setup, dependencies,
quality checks, tests, and development workflows into one cohesive CLI.

## Status

**Early development — Foundation milestone.** Only `pyfaro --help` and
`pyfaro --version` are implemented. There is no published release or supported
installation package yet. The package version `0.1.0-dev.0` identifies this
development scaffold, not a shipped v0.1.0 release.

## Planned experience

- Create and initialize Python projects with opinionated, editable defaults.
- Add, remove, and synchronize dependencies through uv.
- Run programs and development servers in the project environment.
- Lint and format with Ruff, type-check with ty, and test with pytest.
- Combine non-destructive quality checks with clear diagnostics.
- Build Python distributions through uv; later add workspaces and task graphs.

Planned commands: `create`, `init`, `add`, `remove`, `sync`, `run`, `dev`,
`lint`, `fmt`, `type`, `check`, `test`, `build`, and `doctor`.
These commands are roadmap items, not working functionality.

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
linter, type checker, or test engine. The initial code contains only a Clap
CLI boundary; modules will grow when there is functionality to own.
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

Python, uv, Ruff, ty, and pytest are not needed to build this scaffold.
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
is included alongside its import tools. All backlog issues remain open until
their full acceptance criteria have been reviewed.

## Community and security

Contributions are welcome. Read our [contribution guide](CONTRIBUTING.md),
[code of conduct](CODE_OF_CONDUCT.md), and [security policy](SECURITY.md).
General questions belong in
[Discussions](https://github.com/pyfaro/pyfaro/discussions).

## License

Pyfaro is licensed under [Apache-2.0](LICENSE).
