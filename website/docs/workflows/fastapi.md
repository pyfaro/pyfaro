# A FastAPI app

This recipe takes a web API from nothing to a running server and a passing CI
workflow: install the tools, create the project, add a dependency, run the
development server, add a route and its test, run the quality gate, define
tasks, build distributions and check everything on GitHub Actions.

The commands and outputs below come from a real run on Linux. The Windows
variants differ only where the shell does; they are shown in tabs.

## Prerequisites and tested versions

| Tool | Version | Role |
| --- | --- | --- |
| [uv](https://docs.astral.sh/uv/) | 0.9.26 or newer | Environments, locking, building |
| Python | 3.10 or newer | The project's interpreter |
| [FastAPI](https://fastapi.tiangolo.com/) | `>=0.161.1` | The web framework (resolved 0.161.1 in this guide) |
| [Uvicorn](https://www.uvicorn.org/) | `uvicorn[standard]>=0.54.0` | The ASGI server run by `pyfaro dev` (resolved 0.54.0) |
| [Ruff](https://docs.astral.sh/ruff/) | 0.16.10 | `check` (format and lint) |
| [ty](https://docs.astral.sh/ty/) | 0.0.85 | `check` (types) |
| [pytest](https://docs.pytest.org/) | 9.1.1 | `test` |

- The generated project pins Ruff, ty and pytest in its `dev` dependency group
  and declares FastAPI and Uvicorn as runtime dependencies.
- FastAPI's `TestClient` needs an HTTP client. The `dev` group provides
  `httpx2` on Python 3.11 and newer, and `httpx` on Python 3.10.
- Pyfaro needs uv but never installs it, and never downloads Python unless you
  run [`pyfaro python install`](../commands/python.md).
- The guide was run with Python 3.13.11. See also the
  [tested versions](../reference/tested-versions.md) and
  [platform support](../concepts/platform-support.md).

## 1. Install uv and Pyfaro

=== "Linux / macOS"

    ```console
    $ curl -LsSf https://astral.sh/uv/install.sh | sh
    ```

=== "Windows (PowerShell)"

    ```powershell
    powershell -ExecutionPolicy ByPass -c "irm https://astral.sh/uv/install.ps1 | iex"
    ```

!!! warning "This guide needs Pyfaro 0.4.0-alpha.1 or newer"
    `create fastapi`, `dev` and named tasks are new in `0.4.0-alpha.1`, which is
    not published yet; the latest published version is `0.3.0-alpha.1`. Until
    it is, install Pyfaro from the repository (Rust 1.90 or newer), on every
    platform:

    ```console
    $ cargo install --git https://github.com/pyfaro/pyfaro --locked pyfaro
    ```

    Once `0.4.0-alpha.1` is released, the prebuilt archives and
    `cargo install pyfaro --version 0.4.0-alpha.1 --locked` described in
    [Installation](../getting-started/installation.md) work too.

Check both tools:

```console
$ pyfaro --version
pyfaro 0.4.0-alpha.1
$ pyfaro tools uv
uv: /home/you/.local/bin/uv (uv 0.9.26)
```

## 2. Create the project

```console
$ pyfaro create fastapi demo-api
Created FastAPI app at ~/projects/demo-api
$ cd demo-api
```

[`create fastapi`](../commands/create.md) writes files only; it does not run
uv or touch the network. The project is a `src/` layout package with two routes
and their tests:

```text
demo-api/
├── README.md
├── pyproject.toml
├── src/demo_api/
│   ├── __init__.py
│   ├── main.py
│   └── py.typed
└── tests/test_main.py
```

`pyproject.toml` already carries everything the next commands need, including
the entry point that [`pyfaro dev`](../commands/dev.md) runs:

```toml title="pyproject.toml"
[project]
name = "demo-api"
version = "0.1.0"
readme = "README.md"
requires-python = ">=3.10"
dependencies = ["fastapi>=0.161.1", "uvicorn[standard]>=0.54.0"]

[dependency-groups]
dev = ["pytest==9.1.1", "ruff==0.16.10", "ty==0.0.85", "httpx2==2.16.0; python_full_version >= '3.11'", "httpx==0.28.1; python_full_version < '3.11'"]

[tool.pyfaro]
schema-version = 1

[tool.pyfaro.dev]
preset = "fastapi"
entry-point = "demo_api.main:app"
```

(The build system, Ruff and pytest tables are left out here.) `src/demo_api/main.py`
holds the application:

```python title="src/demo_api/main.py"
"""HTTP application for demo-api."""

from fastapi import FastAPI

app = FastAPI(title="demo-api")


@app.get("/")
def read_root() -> dict[str, str]:
    return {"message": "Hello from demo-api!"}


@app.get("/health")
def read_health() -> dict[str, str]:
    return {"status": "ok"}
```

## 3. Sync the environment

```console
$ pyfaro sync
Using CPython 3.13.11
Creating virtual environment at: .venv
Resolved 40 packages in 33ms
   Building demo-api @ file://~/projects/demo-api
      Built demo-api @ file://~/projects/demo-api
Prepared 1 package in 3ms
Installed 31 packages in 27ms
 + annotated-doc==0.0.5
 + annotated-types==0.8.0
 + anyio==4.15.1
 ...
 + fastapi==0.161.1
 ...
 + uvicorn==0.54.0
 ...
```

This creates `.venv` and `uv.lock`. The generated project already passes the
gate, so you can run `pyfaro check` and `pyfaro test` right away.

## 4. Add a dependency

The API will read its greeting from the environment, which is what
[pydantic-settings](https://docs.pydantic.dev/latest/concepts/pydantic_settings/)
is for. [`pyfaro add`](../commands/add-remove.md) edits `pyproject.toml` and the
lock file but does not install, so sync afterwards:

```console
$ pyfaro add pydantic-settings
Resolved 42 packages in 25ms
$ pyfaro sync
Resolved 42 packages in 0.70ms
   Building demo-api @ file://~/projects/demo-api
      Built demo-api @ file://~/projects/demo-api
Prepared 1 package in 3ms
Uninstalled 1 package in 0.13ms
Installed 2 packages in 3ms
 ~ demo-api==0.1.0 (from file://~/projects/demo-api)
 + pydantic-settings==2.16.1
```

`pyproject.toml` now lists it among the runtime dependencies; the exact lower
bound is chosen by uv:

```toml
dependencies = [
    "fastapi>=0.161.1",
    "pydantic-settings>=2.15.0",
    "uvicorn[standard]>=0.54.0",
]
```

## 5. Run the development server

```console
$ pyfaro dev
pyfaro dev: starting demo_api.main:app with uvicorn (reload on) at http://127.0.0.1:8000 - press Ctrl-C to stop
INFO:     Will watch for changes in these directories: ['~/projects/demo-api/src']
INFO:     Uvicorn running on http://127.0.0.1:8000 (Press CTRL+C to quit)
INFO:     Started reloader process [518945] using WatchFiles
INFO:     Started server process [519644]
INFO:     Waiting for application startup.
INFO:     Application startup complete.
```

Lines starting with `pyfaro dev: ` are Pyfaro's, on standard error;
everything else is Uvicorn's own output, passed through unchanged. Pyfaro
adds no file watcher: reloading is Uvicorn's.

Leave it running and, in a second terminal, call the app:

=== "Linux / macOS"

    ```console
    $ curl http://127.0.0.1:8000/health
    {"status":"ok"}
    ```

=== "Windows (PowerShell)"

    ```powershell
    PS> Invoke-RestMethod http://127.0.0.1:8000/health

    status
    ------
    ok
    ```

The interactive documentation is at <http://127.0.0.1:8000/docs>.

**A port that is busy.** If something already listens on port 8000, Pyfaro
stops before launching:

```console
$ pyfaro dev
pyfaro dev: port 8000 on 127.0.0.1 is already in use
pyfaro dev: stop the process using it, run pyfaro dev -- --port 8001, or set args = ["--port", "8001"] in [tool.pyfaro.dev]
$ echo $?
1
```

(In PowerShell the exit status is `$LASTEXITCODE`.) Arguments after `--` go to Uvicorn, so `pyfaro dev -- --port 8001` serves on
another port for this run. To change it for good, set
`args = ["--port", "8001"]` in `[tool.pyfaro.dev]`. See
[`pyfaro dev`](../commands/dev.md) for every setting.

**Stopping.** Press Ctrl-C in the terminal running `pyfaro dev`. Pyfaro stops
the whole process tree (Uvicorn's reloader and its worker), prints
`pyfaro dev: stopped` and exits with status 130. On Linux and macOS it waits
about five seconds for a clean shutdown; on Windows the tree is terminated at
once. No process is left behind, and the port is free again.

## 6. Add a route and its test

Keep `pyfaro dev` running while you edit: the server reloads when you save.

Create `src/demo_api/settings.py`:

```python title="src/demo_api/settings.py"
"""Application settings, read from DEMO_API_* environment variables."""

from pydantic_settings import BaseSettings, SettingsConfigDict


class Settings(BaseSettings):
    model_config = SettingsConfigDict(env_prefix="DEMO_API_")

    greeting: str = "Hello"


def get_settings() -> Settings:
    return Settings()
```

and replace `src/demo_api/main.py` with:

```python title="src/demo_api/main.py"
"""HTTP application for demo-api."""

from typing import Annotated

from fastapi import Depends, FastAPI

from demo_api.settings import Settings, get_settings

app = FastAPI(title="demo-api")


@app.get("/")
def read_root() -> dict[str, str]:
    return {"message": "Hello from demo-api!"}


@app.get("/health")
def read_health() -> dict[str, str]:
    return {"status": "ok"}


@app.get("/hello/{name}")
def read_hello(
    name: str, settings: Annotated[Settings, Depends(get_settings)]
) -> dict[str, str]:
    return {"message": f"{settings.greeting}, {name}!"}
```

In the `pyfaro dev` terminal, Uvicorn notices the change and restarts the
worker:

```text
WARNING:  WatchFiles detected changes in 'src/demo_api/settings.py', 'src/demo_api/main.py'. Reloading...
INFO:     Shutting down
INFO:     Waiting for application shutdown.
INFO:     Application shutdown complete.
INFO:     Finished server process [519644]
INFO:     Started server process [544186]
INFO:     Waiting for application startup.
INFO:     Application startup complete.
```

=== "Linux / macOS"

    ```console
    $ curl http://127.0.0.1:8000/hello/Ada
    {"message":"Hello, Ada!"}
    ```

=== "Windows (PowerShell)"

    ```powershell
    PS> Invoke-RestMethod http://127.0.0.1:8000/hello/Ada

    message
    -------
    Hello, Ada!
    ```

Now the tests. Replace `tests/test_main.py`:

```python title="tests/test_main.py"
"""Smoke tests for the HTTP routes."""

import pytest
from fastapi.testclient import TestClient

from demo_api.main import app

client = TestClient(app)


def test_read_root() -> None:
    response = client.get("/")
    assert response.status_code == 200
    assert response.json() == {"message": "Hello from demo-api!"}


def test_read_health() -> None:
    response = client.get("/health")
    assert response.status_code == 200
    assert response.json() == {"status": "ok"}


def test_read_hello() -> None:
    response = client.get("/hello/Ada")
    assert response.status_code == 200
    assert response.json() == {"message": "Hello, Ada!"}


def test_read_hello_with_custom_greeting(monkeypatch: pytest.MonkeyPatch) -> None:
    monkeypatch.setenv("DEMO_API_GREETING", "Bonjour")
    response = client.get("/hello/Ada")
    assert response.json() == {"message": "Bonjour, Ada!"}
```

Stop the server with Ctrl-C before moving on, or open another terminal.

## 7. Check and test

[`pyfaro check`](../commands/check.md) runs the format check, Ruff and ty;
[`pyfaro test`](../commands/test.md) runs pytest:

```console
$ pyfaro check
5 files already formatted
All checks passed!
All checks passed!

== pyfaro check summary ==
format: ok
lint: ok
type: ok
check: ok
$ pyfaro test
============================= test session starts ==============================
platform linux -- Python 3.13.11, pytest-9.1.1, pluggy-1.7.0
rootdir: ~/projects/demo-api
configfile: pyproject.toml
testpaths: tests
plugins: anyio-4.15.1
collected 4 items

tests/test_main.py ....                                                  [100%]

============================== 4 passed in 0.33s ===============================
```

Both commands are the ones CI runs later. If `check` reports formatting or lint
problems, `pyfaro check --fix` applies the safe fixes; see
[The quality gate](quality-gate.md).

## 8. Define tasks

A [named task](../commands/run.md#named-tasks) gives a short name to a command
you run often. Add this to `pyproject.toml`:

```toml title="pyproject.toml"
[tool.pyfaro.tasks]
openapi = ["python", "-c", "import json; from demo_api.main import app; print(json.dumps(app.openapi(), indent=2))"]
serve = { command = ["uvicorn", "demo_api.main:app", "--port", "8080"], env = { DEMO_API_GREETING = "Welcome" }, description = "Serve without reload, as in production" }
```

An array is the program followed by its arguments. A table can also set `env`,
`cwd` and a `description`. No shell is involved, so the same definitions work on
Linux, macOS and Windows.

Run a task with `pyfaro run <task>`. The first task prints the OpenAPI schema
of the app on standard output (uv's own messages go to standard error):

=== "Linux / macOS"

    ```console
    $ pyfaro run openapi > openapi.json
    $ head -n 8 openapi.json
    {
      "openapi": "3.1.0",
      "info": {
        "title": "demo-api",
        "version": "0.1.0"
      },
      "paths": {
        "/": {
    ```

=== "Windows (PowerShell)"

    ```powershell
    PS> pyfaro run openapi > openapi.json
    PS> Get-Content openapi.json -TotalCount 8
    ```

    Windows PowerShell 5.1 writes `>` redirections as UTF-16; use
    `| Set-Content -Encoding utf8 openapi.json` there (PowerShell 7 writes
    UTF-8).

The second starts Uvicorn without the reloader and with the task's `env`.
Extra arguments after `--` are appended, here to pick another port:

```console
$ pyfaro run serve -- --port 8081
INFO:     Started server process [608218]
INFO:     Waiting for application startup.
INFO:     Application startup complete.
INFO:     Uvicorn running on http://127.0.0.1:8081 (Press CTRL+C to quit)
```

=== "Linux / macOS"

    ```console
    $ curl http://127.0.0.1:8081/hello/Ada
    {"message":"Welcome, Ada!"}
    ```

=== "Windows (PowerShell)"

    ```powershell
    PS> Invoke-RestMethod http://127.0.0.1:8081/hello/Ada

    message
    -------
    Welcome, Ada!
    ```

Ctrl-C stops it (`error: operation cancelled`, exit status 130). A name before
`--` is always a task and a program always follows `--`; an unknown task lists
the defined ones. The rules are in
[`pyfaro run`](../commands/run.md#named-tasks).

## 9. Build distributions

Pyfaro does not have a `build` command yet; `pyfaro build` is planned for
v0.5.0. Build with uv, which is what `pyfaro build` will call:

```console
$ uv build
Building source distribution (uv build backend)...
Building wheel from source distribution (uv build backend)...
Successfully built dist/demo_api-0.1.0.tar.gz
Successfully built dist/demo_api-0.1.0-py3-none-any.whl
```

`dist/` now holds the source distribution and the wheel. Publish them with
`uv publish`, or ship the app another way, for example in a container image.

## 10. Continuous integration

Commit the project with its `uv.lock`, and add a `.gitignore` (`create` does
not write one) that excludes at least `.venv/`, `dist/` and `__pycache__/`.
Then save this as `.github/workflows/ci.yml`. It runs `sync --locked`, `check`
and `test` on Linux and Windows, exactly the commands you ran above. The
[Continuous integration](continuous-integration.md) page explains each step and
the checksum verification in more detail.

=== "cargo install"

    This variant works today, before `0.4.0-alpha.1` is published. Pin
    `--rev <commit>` to make the build reproducible.

    ```yaml title=".github/workflows/ci.yml"
    name: CI

    on:
      push:
        branches: [main]
      pull_request:

    permissions:
      contents: read

    jobs:
      quality:
        runs-on: ${{ matrix.os }}
        strategy:
          fail-fast: false
          matrix:
            os: [ubuntu-latest, windows-latest]
        steps:
          - uses: actions/checkout@v7
            with:
              persist-credentials: false

          - name: Install uv
            uses: astral-sh/setup-uv@v7
            with:
              version: "0.9.26"

          - name: Set up Python
            uses: actions/setup-python@v6
            with:
              python-version: "3.13"

          - name: Install pyfaro
            run: cargo install --git https://github.com/pyfaro/pyfaro --locked pyfaro

          - name: Show tool versions
            run: |
              pyfaro --version
              uv --version

          - name: Sync the locked environment
            run: pyfaro sync --locked

          - name: Check formatting, lint and types
            run: pyfaro check

          - name: Run the tests
            run: pyfaro test
    ```

=== "Release archive"

    For `0.4.0-alpha.1` or newer, once it is published. Set `PYFARO_VERSION` to
    the version you pin. Linux and Windows archives differ, so each platform has
    its own install step.

    ```yaml title=".github/workflows/ci.yml"
    name: CI

    on:
      push:
        branches: [main]
      pull_request:

    permissions:
      contents: read

    jobs:
      quality:
        runs-on: ${{ matrix.os }}
        strategy:
          fail-fast: false
          matrix:
            os: [ubuntu-latest, windows-latest]
        env:
          PYFARO_VERSION: 0.4.0-alpha.1
        steps:
          - uses: actions/checkout@v7
            with:
              persist-credentials: false

          - name: Install uv
            uses: astral-sh/setup-uv@v7
            with:
              version: "0.9.26"

          - name: Set up Python
            uses: actions/setup-python@v6
            with:
              python-version: "3.13"

          - name: Install pyfaro (Linux)
            if: runner.os == 'Linux'
            shell: bash
            run: |
              set -euo pipefail
              name="pyfaro-${PYFARO_VERSION}-x86_64-unknown-linux-gnu"
              url="https://github.com/pyfaro/pyfaro/releases/download/v${PYFARO_VERSION}"
              curl -fsSLO "${url}/${name}.tar.gz"
              curl -fsSLO "${url}/SHA256SUMS"
              grep " ${name}.tar.gz\$" SHA256SUMS | sha256sum --check -
              tar -xzf "${name}.tar.gz"
              mkdir -p "$HOME/.local/bin"
              install "${name}/pyfaro" "$HOME/.local/bin/pyfaro"
              echo "$HOME/.local/bin" >> "$GITHUB_PATH"
              rm -rf SHA256SUMS "${name}" "${name}.tar.gz"

          - name: Install pyfaro (Windows)
            if: runner.os == 'Windows'
            shell: pwsh
            run: |
              $name = "pyfaro-$env:PYFARO_VERSION-x86_64-pc-windows-msvc"
              $url = "https://github.com/pyfaro/pyfaro/releases/download/v$env:PYFARO_VERSION"
              Invoke-WebRequest "$url/$name.zip" -OutFile "$name.zip"
              Invoke-WebRequest "$url/SHA256SUMS" -OutFile SHA256SUMS
              $expected = (Select-String -Path SHA256SUMS -Pattern " $name.zip$").Line.Split(" ")[0]
              $actual = (Get-FileHash "$name.zip" -Algorithm SHA256).Hash.ToLower()
              if ($actual -ne $expected) { throw "SHA-256 mismatch for $name.zip" }
              Expand-Archive "$name.zip" -DestinationPath .
              $bin = Join-Path $env:RUNNER_TEMP "pyfaro-bin"
              New-Item -ItemType Directory -Path $bin -Force | Out-Null
              Copy-Item "$name\pyfaro.exe" $bin
              Add-Content -Path $env:GITHUB_PATH -Value $bin
              Remove-Item -Recurse -Force SHA256SUMS, $name, "$name.zip"

          - name: Show tool versions
            run: |
              pyfaro --version
              uv --version

          - name: Sync the locked environment
            run: pyfaro sync --locked

          - name: Check formatting, lint and types
            run: pyfaro check

          - name: Run the tests
            run: pyfaro test
    ```

`pyfaro sync --locked` fails when `uv.lock` is missing or out of date, so a
stale lock is caught in CI instead of papered over. `check` and `test` do not
sync on their own, which is why `sync` comes first. The `dev` table and the
tasks are not needed in CI.

## Troubleshooting

| Symptom | Fix |
| --- | --- |
| `pyfaro dev: no [tool.pyfaro.dev] table` | The project was not created with `create fastapi`. Add the table shown in [step 2](#2-create-the-project). |
| `pyfaro dev: uvicorn is not declared in the project dependencies` | Run `pyfaro add "uvicorn[standard]"`, then `pyfaro sync`. |
| `port 8000 on 127.0.0.1 is already in use` | Stop the other process or use `pyfaro dev -- --port 8001`. |
| `unknown [tool.pyfaro] setting: dev` (or `tasks`) | Pyfaro older than 0.4.0-alpha.1 does not know `[tool.pyfaro.dev]` or `[tool.pyfaro.tasks]`; upgrade it. |
| `check` or `test` cannot import a package | Run `pyfaro sync` after `pyfaro add`. |

More in [Troubleshooting](troubleshooting.md).

## Next steps

- [`pyfaro dev`](../commands/dev.md) lists every `[tool.pyfaro.dev]` setting.
- [`pyfaro run`](../commands/run.md#named-tasks) covers task `cwd`, shell tasks and quoting.
- [The quality gate](quality-gate.md) adds a pre-commit hook around `check`.
- [Continuous integration](continuous-integration.md) shows a matrix over Python versions.
