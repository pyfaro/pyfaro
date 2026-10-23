# Create a Python project

`pyfaro create` generates a small, packaged Python project (a library, a
command-line application, or a FastAPI application). Creation writes
files locally; it does not run uv, install Python, resolve dependencies, or
download anything. The target's parent directory must already exist, and an
existing file, directory, or symlink is never replaced. This command is
available in published `0.2.0-alpha.1` and later versions; `create fastapi`
is new in `0.4.0-alpha.1`.

## Library

```sh
pyfaro create lib acme-widget --path ./projects/acme-widget --python 3.12
```

The distribution name is required. `--path` defaults to a directory with that
name under the current directory. `--python` defaults to `3.10` and sets the
minimum `requires-python` version; it does not choose or install an interpreter.
Use a name made of ASCII letters, digits, dots, underscores, and hyphens,
excluding reserved Windows device names such as `CON` and `COM1`, and
`-stubs` distribution names that require a different package layout. The
import package is the lowercased name with each run of dots, underscores, or
hyphens replaced by one underscore, so `acme-widget` creates
`src/acme_widget/`. If the distribution
name begins with a digit, the import package gets a leading underscore:
`1widget` creates `src/_1widget/`.

The template contains `pyproject.toml`, `README.md`, a typed package under
`src/`, and a pytest smoke test under `tests/`. It uses the uv build backend
and `[tool.pyfaro] schema-version = 1`.

It also opts in to the tested quality tools, owned by their standard
`pyproject.toml` sections:

```toml
[dependency-groups]
dev = ["pytest==9.1.1", "ruff==0.16.10", "ty==0.0.85"]

[tool.ruff]
line-length = 88
src = ["src", "tests"]

[tool.pytest.ini_options]
testpaths = ["tests"]
```

`ty` needs no configuration for the starter layout, so there is no `[tool.ty]`
section. Edit these sections freely; Pyfaro never overrides them.

## Command-line project

```sh
pyfaro create cli acme-tools --package acme_console --command acme-do \
  --path ./projects/acme-tools --python 3.12
```

`--package` selects the Python import package; by default it is derived from
the distribution name as above. It must be a Python identifier, not a Python
standard-library module, keyword, dunder module such as `__main__`, or
reserved Windows device name. The module inventory covers Python 3.10 through
3.13. This
also applies to the derived default; choose another distribution name if it
would collide. `--command`
selects the installed executable name; by default it is the distribution
name with dots changed to hyphens. It accepts ASCII letters, digits, hyphens,
and underscores, excluding reserved Windows device names. These names
can differ independently. The generated `[project.scripts]` entry points to
`acme_console.cli:main` in this example. When the import package differs from
the distribution's normalized name, the template sets
`[tool.uv.build-backend] module-name` so both wheel and editable installs use
the selected package.

## FastAPI project

```sh
pyfaro create fastapi acme-api --package backend --path ./projects/acme-api \
  --python 3.12
```

The FastAPI template takes the distribution name, `--path`, `--python`, and
`--package`, with the same naming rules, rollback behavior, and
"no uv, no network" guarantee as the other templates. It adds `fastapi` and
`uvicorn[standard]` as runtime dependencies and an HTTP client to the pinned
`dev` group, which `fastapi.testclient.TestClient` needs: `httpx2==2.16.0` on
Python 3.11 and later, where current Starlette prefers it and warns about plain
`httpx`, and `httpx==0.28.1` on Python 3.10:

```toml
dependencies = ["fastapi>=0.161.1", "uvicorn[standard]>=0.54.0"]

[dependency-groups]
dev = ["pytest==9.1.1", "ruff==0.16.10", "ty==0.0.85", "httpx2==2.16.0; python_full_version >= '3.11'", "httpx==0.28.1; python_full_version < '3.11'"]

[tool.pyfaro.dev]
preset = "fastapi"
entry-point = "backend.main:app"
```

The `[tool.pyfaro.dev]` table is what `pyfaro dev` reads (see
[dev.md](dev.md)); its entry point follows `--package`.

`src/<package>/main.py` exposes `app = FastAPI(title="acme-api")` with
`GET /` returning `{"message": "Hello from acme-api!"}` and `GET /health`
returning `{"status": "ok"}`; `tests/test_main.py` covers both routes with
`TestClient`. After `pyfaro sync`, `pyfaro check`, `pyfaro test`, and
`uv build` pass. Serve the application with auto-reload using `pyfaro dev`
(pass Uvicorn options after `--`, for example `pyfaro dev -- --port 8001`), or
check the import with
`pyfaro run -- python -c "from backend.main import app; print(app.title)"`.

## Develop and build

From the generated directory, with a compatible Python interpreter already
installed and network access for the first sync:

```sh
pyfaro sync
pyfaro check
pyfaro test
uv build
```

`pyfaro sync` creates `.venv` and `uv.lock` and installs the pinned Ruff, ty,
and pytest from the `dev` group. Commit `uv.lock` so everyone resolves the same
versions. Creation itself never runs uv, and the quality commands never sync
implicitly, so run `pyfaro sync` before `pyfaro check` and `pyfaro test`. The
generated source passes `pyfaro check` (Ruff format check, Ruff lint with its
default rules, and ty) and `pyfaro test` immediately after the first sync. For
a command-line project, also run `pyfaro run --locked -- acme-do` (substitute
the selected command). uv 0.9.26 is the tested version; its bundled build
backend satisfies the generated `uv_build>=0.9.26,<0.10.0` requirement. A
compatible installed Python is still required.

See [Linting, type checking and check](quality.md) and
[Testing](test.md) for the individual commands.

The layout follows uv's [project templates](https://docs.astral.sh/uv/concepts/projects/init/)
and [build backend module settings](https://docs.astral.sh/uv/concepts/build-backend/).
