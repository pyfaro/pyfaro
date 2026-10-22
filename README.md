<p align="center">
  <img src="https://raw.githubusercontent.com/pyfaro/pyfaro/main/assets/branding/pyfaro-logo.svg" alt="Pyfaro" width="220">
</p>

<h3 align="center">One command-line workflow for Python projects.<br>Fast, opinionated, built with Rust.</h3>

<p align="center">
  <a href="https://crates.io/crates/pyfaro"><img src="https://img.shields.io/crates/v/pyfaro?include_prereleases&label=crates.io" alt="crates.io version"></a>
  <a href="https://github.com/pyfaro/pyfaro/actions/workflows/ci.yml"><img src="https://github.com/pyfaro/pyfaro/actions/workflows/ci.yml/badge.svg" alt="CI status"></a>
  <a href="https://github.com/pyfaro/pyfaro/blob/main/LICENSE"><img src="https://img.shields.io/badge/license-Apache--2.0-blue" alt="License: Apache-2.0"></a>
  <a href="https://pyfaro.github.io/pyfaro/"><img src="https://img.shields.io/badge/docs-pyfaro.github.io-D4A017" alt="Documentation"></a>
</p>

Pyfaro brings project creation, dependencies, Python versions, linting,
formatting, type checking and testing under a single `pyfaro` command. It does
not replace the tools you already trust: it orchestrates
[uv](https://docs.astral.sh/uv/), [Ruff](https://docs.astral.sh/ruff/),
[ty](https://docs.astral.sh/ty/) and [pytest](https://docs.pytest.org/) behind
one consistent, scriptable interface, so a new contributor, a CI job and a
git hook all run exactly the same commands.

> **Alpha software.** Pyfaro is under active development and interfaces may
> change between alpha versions. See [Project status](#project-status).

## Why Pyfaro

- **One entry point.** `create`, `sync`, `add`, `check`, `test` and more,
  instead of remembering four tools and their flags.
- **`pyfaro check` is your quality gate.** Format check, lint and type check
  in one run, with a clear summary and an honest exit code. It writes files
  only when you pass `--fix`.
- **No second ecosystem.** Standard `pyproject.toml` and uv's `uv.lock`.
  Pyfaro adds a tiny `[tool.pyfaro]` table and leaves everything else alone.
- **Adopt it in place.** `pyfaro init` previews what it would change before
  touching your existing project.
- **Predictable by design.** Nothing syncs, installs or downloads behind your
  back. `doctor` explains what is wrong and how to fix it.
- **A single native binary** written in Rust.

## Demo

```console
$ pyfaro create lib textkit
Created library at ~/projects/textkit
$ cd textkit
$ pyfaro sync
Using CPython 3.13.11
Creating virtual environment at: .venv
Resolved 12 packages in 93ms
...
$ pyfaro check
4 files already formatted
All checks passed!
All checks passed!

== pyfaro check summary ==
format: ok
lint: ok
type: ok
check: ok
$ pyfaro test
============================= test session starts ==============================
...
collected 1 item

tests/test_library.py .                                                  [100%]

============================== 1 passed in 0.01s ===============================
```

## Installation

Pyfaro needs [uv](https://docs.astral.sh/uv/getting-started/installation/)
**0.9.26 or newer** at runtime. Pyfaro never installs uv or Python for you.

**From crates.io** (Rust 1.90 or newer):

```sh
cargo install pyfaro --version 0.3.0-alpha.1 --locked
```

While every published version is an alpha, Cargo needs the explicit
`--version`; a bare `cargo install pyfaro` finds no stable release.

**Prebuilt binaries** for Linux (x86_64, aarch64), macOS (arm64, x86_64) and
Windows (x86_64) are attached to each
[GitHub Release](https://github.com/pyfaro/pyfaro/releases), together with a
`SHA256SUMS` file. The archives are unsigned: verify them against
`SHA256SUMS`, extract, and put `pyfaro` on your `PATH`.

Check the installation:

```sh
pyfaro --version
```

More detail: [Installation](https://pyfaro.github.io/pyfaro/getting-started/installation/).

## Quick start

```sh
pyfaro create lib my-lib      # or: pyfaro create cli my-app
cd my-lib
pyfaro sync                   # create .venv and uv.lock
pyfaro add "requests>=2"      # then: pyfaro sync
pyfaro check                  # format check, lint, type check
pyfaro test                   # run pytest
```

Adopting an existing project? Preview first, then apply (Linux only for now):

```sh
pyfaro init                   # preview, writes nothing
pyfaro init --apply
```

Walk through it step by step in
[First steps](https://pyfaro.github.io/pyfaro/getting-started/first-steps/), or
follow a full [workflow](https://pyfaro.github.io/pyfaro/workflows/).

## Commands

| Command | What it does |
| --- | --- |
| [`create`](https://pyfaro.github.io/pyfaro/commands/create/) | Scaffold a packaged library (`lib`) or command-line app (`cli`) |
| [`init`](https://pyfaro.github.io/pyfaro/commands/init/) | Preview or apply Pyfaro initialization of an existing project |
| [`sync`](https://pyfaro.github.io/pyfaro/commands/sync/) | Create or update `.venv` and `uv.lock` through uv |
| [`add` / `remove`](https://pyfaro.github.io/pyfaro/commands/add-remove/) | Edit dependencies and dependency groups through uv |
| [`run`](https://pyfaro.github.io/pyfaro/commands/run/) | Run a program in the project environment |
| [`python`](https://pyfaro.github.io/pyfaro/commands/python/) | List, install and pin Python versions through uv |
| [`tools`](https://pyfaro.github.io/pyfaro/commands/tools/) | Show the resolved path and version of uv, Ruff, ty and pytest |
| [`doctor`](https://pyfaro.github.io/pyfaro/commands/doctor/) | Diagnose configuration, environment and tools |
| [`lint`](https://pyfaro.github.io/pyfaro/commands/lint/) / [`fmt`](https://pyfaro.github.io/pyfaro/commands/fmt/) | Lint and format with Ruff |
| [`type`](https://pyfaro.github.io/pyfaro/commands/type/) | Type-check with ty |
| [`check`](https://pyfaro.github.io/pyfaro/commands/check/) | Format check, lint and type check in one run; `--fix` applies safe fixes |
| [`test`](https://pyfaro.github.io/pyfaro/commands/test/) | Run pytest in the project environment |

Recipes live in the [workflows](https://pyfaro.github.io/pyfaro/workflows/)
section: a [new library](https://pyfaro.github.io/pyfaro/workflows/new-library/),
a [new CLI](https://pyfaro.github.io/pyfaro/workflows/new-cli/),
[adopting an existing project](https://pyfaro.github.io/pyfaro/workflows/adopt-existing-project/),
[CI](https://pyfaro.github.io/pyfaro/workflows/continuous-integration/) and
[troubleshooting](https://pyfaro.github.io/pyfaro/workflows/troubleshooting/).

## How it works

Pyfaro owns the command line, configuration, orchestration and diagnostics.
Everything else stays with the tool built for it.

| Responsibility | Owner |
| --- | --- |
| CLI, configuration, orchestration, diagnostics | Pyfaro |
| Python, environments, resolution, dependencies, packaging | uv |
| Linting and formatting | Ruff |
| Static typing | ty |
| Test execution | pytest |

There is no second resolver, lock file format, package manager, linter or test
engine: Pyfaro calls the real tools and passes their output and exit status
through. Read more in
[How it works](https://pyfaro.github.io/pyfaro/concepts/how-it-works/) and the
[architecture decision record](docs/architecture/0001-project-architecture.md).

## Project status

Pyfaro is **alpha**. The latest published version is
[`0.3.0-alpha.1`](https://crates.io/crates/pyfaro/0.3.0-alpha.1); the `main`
branch develops `0.4.0-alpha.1`. Interfaces may change between alpha versions.

- **Tested versions:** uv 0.9.26, Ruff 0.16.10, ty 0.0.85, pytest 9.1.1.
- **Platforms:** Linux, macOS and Windows are built and tested in CI.
  `init --apply` is **Linux-only** for now; on macOS and Windows `init`
  previews but does not apply.
- **Implemented:** `create`, `init`, `sync`, `add`, `remove`, `run`, `python`,
  `tools`, `doctor`, `lint`, `fmt`, `type`, `check` and `test`.

See the [changelog](CHANGELOG.md) for what each release contains.

## Roadmap

Planned, **not yet available**: `pyfaro dev` (development mode), `pyfaro build`
(distributions through uv), and later workspaces and tasks. Milestones are
planning targets, not release announcements. Follow the
[roadmap board](https://github.com/orgs/pyfaro/projects/1),
[milestones](https://github.com/pyfaro/pyfaro/milestones) and
[issues](https://github.com/pyfaro/pyfaro/issues).

## Contributing

Contributions are welcome. Read the [contribution guide](CONTRIBUTING.md) and
the [code of conduct](CODE_OF_CONDUCT.md); report vulnerabilities as described
in the [security policy](SECURITY.md). Questions are welcome in
[Discussions](https://github.com/pyfaro/pyfaro/discussions).

To build from source you need Rust (the toolchain is pinned in
`rust-toolchain.toml`):

```sh
git clone https://github.com/pyfaro/pyfaro.git
cd pyfaro
cargo build --locked
cargo run --locked -- --help
```

## License

Pyfaro is licensed under [Apache-2.0](LICENSE).
