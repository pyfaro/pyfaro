<p align="center">
  <img src="https://raw.githubusercontent.com/pyfaro/pyfaro/main/assets/branding/pyfaro-logo.svg" alt="Pyfaro" width="220">
</p>

<h3 align="center">One command-line workflow for Python projects.<br>Fast, opinionated, built with Rust.</h3>

<p align="center">
  <a href="https://crates.io/crates/pyfaro"><img src="https://img.shields.io/crates/v/pyfaro?include_prereleases&label=crates.io" alt="crates.io version"></a>
  <a href="https://github.com/pyfaro/pyfaro/actions/workflows/ci.yml"><img src="https://github.com/pyfaro/pyfaro/actions/workflows/ci.yml/badge.svg" alt="CI status"></a>
  <a href="https://github.com/pyfaro/pyfaro/blob/main/LICENSE"><img src="https://img.shields.io/badge/license-Apache--2.0-blue" alt="License: Apache-2.0"></a>
  <a href="https://pyfaro.dev/"><img src="https://img.shields.io/badge/docs-pyfaro.dev-D4A017" alt="Documentation"></a>
</p>

Pyfaro brings project creation, dependencies, Python versions, linting,
formatting, type checking and testing under a single `pyfaro` command. It does
not replace the tools you already trust: it orchestrates
[uv](https://docs.astral.sh/uv/), [Ruff](https://docs.astral.sh/ruff/),
[ty](https://docs.astral.sh/ty/) and [pytest](https://docs.pytest.org/) behind
one consistent, scriptable interface, so a new contributor, a CI job and a
git hook all run exactly the same commands.

> **Beta software.** Pyfaro is under active development and interfaces may
> still change before 1.0. See [Project status](#project-status).

## Why Pyfaro

- **One entry point.** `create`, `sync`, `add`, `check`, `test` and more,
  instead of remembering four tools and their flags.
- **A quality gate with no setup.** `pyfaro check` runs the format check, lint
  and type check, always runs all three and reports them in one summary. It
  writes files only with `--fix`.
- **From scaffold to running server.** `pyfaro create fastapi` generates a
  typed app and `pyfaro dev` runs it with reload; Ctrl-C stops the whole
  process tree.
- **No second ecosystem.** Standard `pyproject.toml` and uv's `uv.lock`, plus
  a tiny `[tool.pyfaro]` table. Named tasks, `init` for existing projects and
  a `doctor` command are included.
- **Predictable.** Nothing syncs, installs or downloads behind your back. A
  single native binary written in Rust.

Wondering how it compares with plain uv, a Makefile or `just`, poethepoet,
Hatch or PDM, and what it does not do yet? Read
[Why Pyfaro](https://pyfaro.dev/getting-started/why-pyfaro/).

## Demo

<p align="center">
  <img src="https://raw.githubusercontent.com/pyfaro/pyfaro/main/website/docs/assets/demo.svg" alt="A terminal recording: pyfaro create fastapi acme-api, then pyfaro sync, pyfaro check and pyfaro test, which all pass. pyfaro dev serves the app with Uvicorn, reloads it after a source file changes and stops on Ctrl-C with exit status 130." width="860">
</p>

The same commands as text, from creating a FastAPI project to the dev server:

```sh
pyfaro create fastapi acme-api   # scaffold a typed FastAPI project
cd acme-api
pyfaro sync                      # create .venv and uv.lock
pyfaro check                     # format check, lint, type check
pyfaro test                      # run pytest
pyfaro dev                       # Uvicorn with reload; Ctrl-C stops the whole tree
```

Recorded on Linux with `0.5.0-beta.1`, from
[a script](https://github.com/pyfaro/pyfaro/blob/main/demo/record.py) that
types the commands and runs the real binary. The edit that makes the server
reload is made by that script, not typed.

## Installation

Pyfaro needs [uv](https://docs.astral.sh/uv/getting-started/installation/)
**0.9.26 or newer** at runtime. Pyfaro never installs uv or Python for you.

**Install script** for Linux and macOS. It downloads the newest release for
your machine over HTTPS, verifies it against `SHA256SUMS` and installs it to
`~/.local/bin`, without `sudo` and without touching your shell profile:

```sh
curl --proto '=https' --tlsv1.2 -LsSf https://pyfaro.dev/install.sh | sh
```

On Windows, in PowerShell:

```powershell
powershell -ExecutionPolicy ByPass -c "irm https://pyfaro.dev/install.ps1 | iex"
```

Prefer to read a script before running it? Download it first with `curl ... -o
install.sh` (or `irm ... -OutFile install.ps1`), inspect it, then run the file.
Run the script again to upgrade; set `PYFARO_VERSION` to pin a version.

**From PyPI**, without a Rust toolchain (from `0.5.0b1`; Pyfaro's
`0.5.0-beta.1` is `0.5.0b1` in Python's version format):

```sh
uv tool install pyfaro      # or: pipx install pyfaro / pip install pyfaro
```

Every published version is still a pre-release, and installers fall back to
the newest pre-release when there is no stable one, so these commands work
as they are. To pin one, name it (`uv tool install pyfaro==0.5.0b1`). Once a
stable version exists, add `--prerelease allow` (uv) or `--pre` (pip, and
`pipx install --pip-args=--pre`) to keep following the pre-releases.

**From crates.io** (Rust 1.90 or newer):

```sh
cargo install pyfaro --version 0.5.0-beta.1 --locked
```

While every published version is a pre-release, Cargo needs the explicit
`--version`; a bare `cargo install pyfaro` finds no stable release.

**Prebuilt binaries** for Linux (x86_64, aarch64), macOS (arm64, x86_64) and
Windows (x86_64) are attached to each
[GitHub Release](https://github.com/pyfaro/pyfaro/releases), together with a
`SHA256SUMS` file and a build provenance attestation for each asset. The
archives are not code-signed: verify them against `SHA256SUMS` (and, with the
GitHub CLI, `gh attestation verify <file> --repo pyfaro/pyfaro`), extract,
and put `pyfaro` on your `PATH`.

**macOS note.** The install script, `uv tool install`, `pipx`, `pip` and
`cargo install` produce files macOS does not quarantine, so Gatekeeper stays
out of the way. Only an archive downloaded in a browser carries the
`com.apple.quarantine` attribute; if macOS refuses to open it, run
`xattr -d com.apple.quarantine pyfaro` on the extracted binary. See
[macOS and Gatekeeper](https://pyfaro.dev/getting-started/installation/#macos-and-gatekeeper).

Check the installation, and the toolchain around it:

```sh
pyfaro --version
pyfaro doctor      # explains a missing or outdated uv and how to fix it
```

More detail: [Installation](https://pyfaro.dev/getting-started/installation/).

## Quick start

```sh
pyfaro create lib my-lib      # or: create cli my-app, create fastapi my-api
cd my-lib
pyfaro sync                   # create .venv and uv.lock
pyfaro add "requests>=2"      # then: pyfaro sync
pyfaro check                  # format check, lint, type check
pyfaro test                   # run pytest
```

Building an API? A FastAPI project is ready to serve right after `sync`
(published in `0.4.0-alpha.1`):

```sh
pyfaro create fastapi my-api
cd my-api
pyfaro sync
pyfaro dev                    # Uvicorn with reload on http://127.0.0.1:8000
```

The [FastAPI guide](https://pyfaro.dev/workflows/fastapi/) goes from install to
a running app and a passing CI workflow.

Adopting an existing project? Preview first, then apply:

```sh
pyfaro init                   # preview, writes nothing
pyfaro init --apply
```

Walk through it step by step in
[First steps](https://pyfaro.dev/getting-started/first-steps/), or
follow a full [workflow](https://pyfaro.dev/workflows/).

## Commands

| Command | What it does |
| --- | --- |
| [`create`](https://pyfaro.dev/commands/create/) | Scaffold a packaged library (`lib`), command-line app (`cli`) or FastAPI app (`fastapi`) |
| [`init`](https://pyfaro.dev/commands/init/) | Preview or apply Pyfaro initialization of an existing project |
| [`sync`](https://pyfaro.dev/commands/sync/) | Create or update `.venv` and `uv.lock` through uv |
| [`add` / `remove`](https://pyfaro.dev/commands/add-remove/) | Edit dependencies and dependency groups through uv |
| [`run`](https://pyfaro.dev/commands/run/) | Run a program, or a named task, in the project environment |
| [`dev`](https://pyfaro.dev/commands/dev/) | Run the development server or process with supervision |
| [`python`](https://pyfaro.dev/commands/python/) | List, install and pin Python versions through uv |
| [`build`](https://pyfaro.dev/commands/build/) | Build a source distribution and a wheel through uv, with the project's own build backend |
| [`tools`](https://pyfaro.dev/commands/tools/) | Show the resolved path and version of uv, Ruff, ty and pytest |
| [`doctor`](https://pyfaro.dev/commands/doctor/) | Diagnose configuration, environment and tools |
| [`lint`](https://pyfaro.dev/commands/lint/) / [`fmt`](https://pyfaro.dev/commands/fmt/) | Lint and format with Ruff |
| [`type`](https://pyfaro.dev/commands/type/) | Type-check with ty |
| [`check`](https://pyfaro.dev/commands/check/) | Format check, lint and type check in one run; `--fix` applies safe fixes |
| [`test`](https://pyfaro.dev/commands/test/) | Run pytest in the project environment |
| [`completions`](https://pyfaro.dev/commands/completions/) | Print the tab-completion script for Bash, Zsh, Fish or PowerShell |

Recipes live in the [workflows](https://pyfaro.dev/workflows/)
section: a [new library](https://pyfaro.dev/workflows/new-library/),
a [new CLI](https://pyfaro.dev/workflows/new-cli/),
a [FastAPI app](https://pyfaro.dev/workflows/fastapi/),
[adopting an existing project](https://pyfaro.dev/workflows/adopt-existing-project/),
[CI](https://pyfaro.dev/workflows/continuous-integration/) and
[troubleshooting](https://pyfaro.dev/workflows/troubleshooting/).

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
[How it works](https://pyfaro.dev/concepts/how-it-works/) and the
[architecture decision record](docs/architecture/0001-project-architecture.md).

## Project status

Pyfaro is **beta**. The latest version is
[`0.5.0-beta.1`](https://crates.io/crates/pyfaro/0.5.0-beta.1) (`0.5.0b1` on
[PyPI](https://pypi.org/project/pyfaro/)). Interfaces may still change before
1.0.

- **Tested versions:** uv 0.9.26, Ruff 0.16.10, ty 0.0.85, pytest 9.1.1.
- **Platforms:** Linux, macOS and Windows are built and tested in CI,
  including `init --apply` with its rollback and cancellation guarantees.
- **Implemented:** `create`, `init`, `sync`, `add`, `remove`, `run`, `dev`, `python`,
  `build`, `tools`, `doctor`, `lint`, `fmt`, `type`, `check`, `test`
  and `completions`.

See the [changelog](CHANGELOG.md) for what each release contains.

## Roadmap

Planned, **not yet available**: workspaces. Milestones are
planning targets, not release announcements. Follow the
[roadmap board](https://github.com/orgs/pyfaro/projects/1),
[milestones](https://github.com/pyfaro/pyfaro/milestones) and
[issues](https://github.com/pyfaro/pyfaro/issues).

## Contributing

Contributions are welcome. Read the [contribution guide](CONTRIBUTING.md) and
the [code of conduct](CODE_OF_CONDUCT.md); report vulnerabilities as described
in the [security policy](SECURITY.md).

Where to go with what:

- Questions and ideas: [Discussions](https://github.com/pyfaro/pyfaro/discussions)
  ([Q&A](https://github.com/pyfaro/pyfaro/discussions/categories/q-a),
  [Ideas](https://github.com/pyfaro/pyfaro/discussions/categories/ideas),
  [Show and tell](https://github.com/pyfaro/pyfaro/discussions/categories/show-and-tell)).
- Bugs: the [bug report form](https://github.com/pyfaro/pyfaro/issues/new?template=bug_report.yml)
  (it asks for `pyfaro --version`, your OS, uv version and `pyfaro -v` output).
- Feature proposals: the [feature request form](https://github.com/pyfaro/pyfaro/issues/new?template=feature_request.yml).
- How the beta went for you: the [beta feedback form](https://github.com/pyfaro/pyfaro/issues/new?template=beta_feedback.yml).

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
