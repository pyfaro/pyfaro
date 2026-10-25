---
description: "What Pyfaro sends, what it runs, where the programs it runs come from, and how to report a vulnerability."
---

# Security and privacy

What Pyfaro sends, what it runs, where the programs it runs come from, and
what you are trusting when you install it. Pyfaro is a thin layer over other
tools, so the page keeps two things apart: what **Pyfaro** does, and what the
**tools it starts** do.

## In short

- Pyfaro sends **no telemetry** and makes **no network connections of its
  own**. It does not check for updates and never updates itself or any tool.
- Network access happens only inside the tools Pyfaro starts: mostly **uv**,
  when it resolves, downloads packages or downloads Python. `--offline`
  switches that off.
- Pyfaro runs programs you already have or that your project names: uv, Ruff,
  ty, pytest, your tasks and your dev server. **A repository you do not trust
  is code you do not trust**, exactly as with `uv run`.
- Pyfaro never installs uv. The install scripts download only from GitHub and
  stop when a checksum does not match.

## Pyfaro does not phone home

Pyfaro has no telemetry, crash reporting, usage statistics, update check or
analytics, and no setting that turns any of them on. It keeps no global state:
it reads your project and writes only the project files a command documents
(and temporary files that it removes).

This is checked, not just promised:

- **The dependency graph has no network client.** `Cargo.lock` contains no HTTP
  or TLS client, no telemetry or error-reporting crate and no self-update crate.
  The test `tests/privacy.rs` fails the build if one appears, so adding one
  forces a change to this page.
- **The sources open no outgoing socket.** The only socket Pyfaro opens is a
  local listener that `pyfaro dev` binds to see whether your port is free, and
  it is closed at once.
- **`--verbose` is local.** It prints tool versions and command lines to
  standard error. Credentials in URLs are masked and environment variable
  values are never printed.

!!! note "tokio on Windows"
    On Windows, the Job Object crate Pyfaro uses to clean up process trees
    depends on `tokio` with its `process` feature. Pyfaro does not use `tokio`
    itself, and nothing in it opens a connection; the test above pins `tokio`
    to that one dependent.

## Network use by command

Network access belongs to the delegated tool. This table says which commands
can cause any, and which tool does it. "Index" means the package index uv is
configured to use (PyPI unless your `pyproject.toml`, `uv.toml` or
environment say otherwise). The behavior was checked against uv 0.9.26 by
pointing uv's index and Python mirror at a local server and recording what it
was asked for.

| Command | Network access | Who |
| --- | --- | --- |
| `create` | None. It never runs uv. | - |
| `init` (preview) | None. | - |
| `init --apply` (Linux) | Reads the index to lock dependencies (`uv lock --no-build`). | uv |
| `sync` | Reads the index if the lock must change; downloads packages missing from uv's cache. | uv |
| `add`, `remove` | Reads the index to resolve and lock. They pass `--no-sync`, so nothing is installed. | uv |
| `run`, `dev` | uv locks and syncs the environment first: index reads and package downloads, as for `sync`. Then your program runs and can do anything. | uv, your code |
| `python list` | None: uv lists a catalog built into the executable (no request was made, online or offline). | - |
| `python install` | Downloads the interpreter. By default from the `python-build-standalone` releases on GitHub. | uv |
| `python pin` | None. | - |
| `lint`, `fmt`, `type`, `check`, `test` | None from Pyfaro. It starts Ruff, ty or pytest from the project environment directly, without uv. What your tests do is up to them. | the tool |
| `build` | With the `uv_build` backend, none: uv builds the project itself. Other backends (hatchling, setuptools, ...) are installed from the index into an isolated build environment, as for any PEP 517 build. | uv |
| `tools`, `doctor` | None. They run version probes, and `doctor` asks uv to validate its configuration with `--offline`. | - |
| `completions` | None. | - |

`sync --locked`, `sync --frozen` and the same flags on `run` and `dev` control
whether uv may update `uv.lock`. Packages missing from uv's cache are still
downloaded.

### Controlling network access

| Control | Effect |
| --- | --- |
| `--offline` (on `init`, `sync`, `add`, `remove`, `run`, `dev`, `python`) | Passes `--offline` to uv: nothing is fetched, and uv fails if the cache lacks what it needs. Applies to `python install` too. |
| `UV_OFFLINE=1` | uv reads it too; the same effect for every command that calls uv. |
| `--no-python-downloads` | Pyfaro always passes it except for `python install` (and `python list`, which downloads nothing). `UV_PYTHON_DOWNLOADS=automatic` therefore cannot make `sync`, `run`, `dev` or `add` fetch an interpreter: they fail and say a download is available. |
| `UV_PYTHON_DOWNLOADS=never` | Also blocks `python install`. |
| `UV_INDEX_URL`, `UV_PYTHON_INSTALL_MIRROR`, `[tool.uv]` | uv's own settings choose the hosts it talks to. Pyfaro leaves them alone. |

## What Pyfaro runs

Pyfaro starts processes without a shell, with your privileges.

| Program | Where it comes from |
| --- | --- |
| **uv** | Installed by you. Pyfaro finds it on `PATH` or takes `--uv <path>`, runs `uv --version` to check it, and never installs or updates it. |
| **Ruff, ty, pytest** | Normal packages in your project environment, resolved by uv from the configured index. `pyfaro create` pins them in the `dev` group (`ruff==0.16.10`, `ty==0.0.85`, `pytest==9.1.1`) and `uv.lock` records the file hashes uv checks. `pyfaro add --dev ruff` takes whatever the index resolves. Pyfaro looks for each tool in the project environment first, then `VIRTUAL_ENV`, then `PATH`. |
| **Python** | An interpreter on your machine, or one you asked uv to download with `pyfaro python install`. |
| **Template dependencies** | `create fastapi` adds `fastapi`, `uvicorn[standard]` and a test HTTP client at pinned versions. The project's own build backend, `uv_build`, ships inside uv: building the project made no index request. |
| **Your project's code** | `[tool.pyfaro.tasks]`, `[tool.pyfaro.dev]`, `pyfaro run`, `pyfaro dev` and `pyfaro test` run commands and code that the project defines. |

Even `pyfaro tools` and `pyfaro doctor` execute the project environment's
`ruff`, `ty` and `pytest` with `--version`, and `doctor` runs the environment's
Python in isolated mode to read package metadata.

### Untrusted repositories

Pyfaro does not sandbox anything. After cloning a repository you do not trust:

- Do not run `sync`, `run`, `dev`, `test` or `check` before reading
  `pyproject.toml`, `uv.toml`, the `[tool.pyfaro.tasks]` and `[tool.pyfaro.dev]`
  tables and the lock file. They install packages (building source
  distributions can run arbitrary code), choose indexes and run the project's
  commands.
- A `.venv` shipped inside the repository is looked at first when Pyfaro finds
  tools. Delete it, or do not run any command in that directory.
- `pyfaro create` and `pyfaro init` (preview) only read and write local files.
- If in doubt, work in a container or a throwaway virtual machine.

## Updates

Pyfaro never updates itself, uv, Ruff, ty, pytest or Python, and never checks
whether a newer version exists. Every upgrade is an explicit action per
install method; see [Upgrade](../getting-started/installation.md#upgrade).
Tool versions change only when you edit the pins in `pyproject.toml` and run
`pyfaro sync`.

## Installing Pyfaro

The [install scripts](../getting-started/installation.md#install-with-the-script)
talk to GitHub only (the release download hosts and `api.github.com` for the
newest-release lookup). They download over HTTPS, verify the archive against
`SHA256SUMS` and install nothing if the check cannot be done or fails. They
never use `sudo`, edit your profile or `PATH`, or install uv.

What backs a release:

- **`SHA256SUMS`** lists every asset. It catches corrupt downloads and bad
  mirrors, but it comes from the same place as the archive, so it does not say
  who built it.
- **Build provenance attestations.** From `0.5.0-beta.1` on, every archive and
  the `.crate` carries a signed GitHub attestation naming the repository,
  workflow and commit that built it. Check one with
  `gh attestation verify <file> --repo pyfaro/pyfaro`. Earlier releases have
  none.
- **Reproducible builds.** Binaries are built twice on separate runners and
  the release fails unless both are identical; the steps to reproduce one are
  in the [release guide](https://github.com/pyfaro/pyfaro/blob/main/docs/releasing.md).
- **crates.io trusted publishing.** The crate is published from the release
  workflow with a short-lived credential, after a maintainer approves the
  deployment. There is no long-lived token to leak.
- **Unsigned binaries.** The archives are not code-signed or notarized. On
  macOS, Gatekeeper may block a binary you downloaded in a browser; the install
  scripts avoid that because `curl` does not mark files as quarantined. On
  Windows, SmartScreen may warn.

`cargo install pyfaro --locked` builds from source with the dependency
versions of the lock file, which you can read in the repository.

## Reporting a problem

Report vulnerabilities privately, as described in the
[security policy](https://github.com/pyfaro/pyfaro/blob/main/SECURITY.md).
Only the latest published version is supported during the public beta, and
fixes land on `main` first. Vulnerabilities in uv, Ruff, ty or pytest belong to
those projects as well.
