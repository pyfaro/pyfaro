# pyfaro create

`pyfaro create` generates a small, packaged Python project: a **library**
(`create lib`), an installable **command-line application** (`create cli`) or
a **FastAPI application** (`create fastapi`).
Use it to start a new project that already has a `src/` layout, a smoke test,
and the quality tools wired into `pyproject.toml`. Creation only writes files.
It never runs uv, installs Python, resolves dependencies or touches the
network.

## Quick example

```console
$ pyfaro create lib acme-widget --python 3.12
Created library at ~/work/acme-widget
$ cd acme-widget
$ pyfaro sync
$ pyfaro check
$ pyfaro test
```

## Library, command-line or FastAPI project?

| | `create lib` | `create cli` | `create fastapi` |
| --- | --- | --- | --- |
| Purpose | A package other code imports | A package that installs an executable | A web API served by uvicorn |
| Extra options | none | `--package`, `--command` | `--package` |
| `[project.scripts]` | absent | present | absent |
| Runtime dependencies | none | none | `fastapi`, `uvicorn[standard]` |

All subcommands take the distribution name as their only argument, plus
`--path` and `--python`.

## Scenarios

### 1. Create a library

```console
$ pyfaro create lib acme-widget --python 3.12
Created library at ~/work/acme-widget
```

The generated tree:

```text
acme-widget/
├── pyproject.toml
├── README.md
├── src/acme_widget/
│   ├── __init__.py
│   └── py.typed
└── tests/
    └── test_library.py
```

### 2. Choose the directory with `--path`

By default the project goes into a directory named after the project, under
the current directory. `--path` picks another target:

```console
$ pyfaro create lib acme-widget --path ./projects/acme-widget
Created library at ./projects/acme-widget
```

The **parent** of the target must already exist. Pyfaro creates the last
directory only, never the whole chain:

```console
$ pyfaro create lib acme-widget --path no/such/parent/acme-widget
error: target parent must be an existing directory: ./no/such/parent
```

### 3. Set the minimum Python with `--python`

`--python MAJOR.MINOR` becomes the lower bound of `requires-python`. The
default is `3.10`.

```console
$ pyfaro create lib acme-widget --python 3.12
$ grep requires-python acme-widget/pyproject.toml
requires-python = ">=3.12"
```

!!! note
    `--python` only writes the constraint. It does not choose, install or pin
    an interpreter. Use [`pyfaro python`](python.md) for that.

A value that is not `MAJOR.MINOR` is rejected:

```console
$ pyfaro create lib acme-widget --python 3
error: Python version must be MAJOR.MINOR, for example 3.12
```

### 4. Create a command-line project

```console
$ pyfaro create cli acme-tools --package acme_console --command acme-do \
    --path ./projects/acme-tools --python 3.12
Created CLI at ./projects/acme-tools
```

- `--package` is the import package (the directory under `src/`). Default:
  the normalized distribution name.
- `--command` is the executable name that gets installed. Default: the
  distribution name with dots replaced by hyphens.

The two can differ from the distribution name, and from each other. The
generated `pyproject.toml` contains:

```toml
[project.scripts]
acme-do = "acme_console.cli:main"

[tool.uv.build-backend]
module-name = "acme_console"
```

`module-name` is only written when the package differs from the normalized
distribution name, so both wheels and editable installs use the package you
chose.

Without the options, the names are derived:

```console
$ pyfaro create cli my.tool
$ grep -A1 scripts my.tool/pyproject.toml
[project.scripts]
my-tool = "my_tool.cli:main"
$ ls my.tool/src
my_tool
```

The CLI project contains `src/<package>/cli.py` with a `main()` function that
prints a greeting and returns `0`, and `tests/test_cli.py` that calls it.

### 5. Create a FastAPI project

```console
$ pyfaro create fastapi acme-api --python 3.12
Created FastAPI app at ~/work/acme-api
```

The generated tree:

```text
acme-api/
├── pyproject.toml
├── README.md
├── src/acme_api/
│   ├── __init__.py
│   ├── main.py
│   └── py.typed
└── tests/
    └── test_main.py
```

`src/acme_api/main.py` exposes `app = FastAPI(title="acme-api")` with two
routes: `GET /` returns `{"message": "Hello from acme-api!"}` and
`GET /health` returns `{"status": "ok"}`. `tests/test_main.py` checks both
with FastAPI's `TestClient`. `--package` works as for `create cli` and adds
`[tool.uv.build-backend] module-name` when it differs from the normalized name.

The `pyproject.toml` adds the application dependencies, and the HTTP client
that `TestClient` needs, to the pinned `dev` group. Python 3.11 and later get
`httpx2` (current Starlette prefers it and warns about plain `httpx`); Python
3.10 gets `httpx`:

```toml
dependencies = ["fastapi>=0.161.1", "uvicorn[standard]>=0.54.0"]

[dependency-groups]
dev = ["pytest==9.1.1", "ruff==0.16.10", "ty==0.0.85", "httpx2==2.16.0; python_full_version >= '3.11'", "httpx==0.28.1; python_full_version < '3.11'"]
```

Sync once, then check, test and serve the application:

```console
$ cd acme-api
$ pyfaro sync
$ pyfaro check
$ pyfaro test
$ pyfaro run --locked -- uvicorn acme_api.main:app --reload
```

`create fastapi` is new in `0.4.0-alpha.1`.

## Naming rules

The **distribution name** must:

- use ASCII letters, digits, dots, underscores and hyphens;
- start and end with a letter or digit;
- not be a reserved Windows device name (`CON`, `COM1`, and so on);
- not end in `-stubs` (those need a different package layout).

The **import package** is the lowercased name with each run of dots,
underscores or hyphens replaced by one underscore, so `A.B_c--d` creates
`src/a_b_c_d/`. A name that starts with a digit gets a leading underscore:
`1widget` creates `src/_1widget/`.

An explicit `--package` must be a Python identifier that is not a keyword, a
standard-library module name (Python 3.10 to 3.13), a dunder module such as
`__main__`, or a reserved Windows device name. The same check applies to the
derived default, so pick another distribution name if it collides.

`--command` accepts ASCII letters, digits, hyphens and underscores, minus
reserved Windows device names. `create fastapi` has no `--command`.

## What the generated project contains

The template uses the uv build backend and opts in to Pyfaro and to the tested
quality tools, each owned by its standard `pyproject.toml` section:

```toml
[project]
name = "acme-widget"
version = "0.1.0"
readme = "README.md"
requires-python = ">=3.12"
dependencies = []

[build-system]
requires = ["uv_build>=0.9.26,<0.10.0"]
build-backend = "uv_build"

[dependency-groups]
dev = ["pytest==9.1.1", "ruff==0.16.10", "ty==0.0.85"]

[tool.pyfaro]
schema-version = 1

[tool.ruff]
line-length = 88
src = ["src", "tests"]

[tool.pytest.ini_options]
testpaths = ["tests"]
```

- The dev tools are **pinned** to the versions Pyfaro is tested with.
- `ty` needs no configuration for the starter layout, so there is no
  `[tool.ty]` section.
- Edit these sections freely. Pyfaro never overrides them.

## Project selection

`create` does not use project discovery. The name argument and `--path`
decide where files go, and the current directory only matters when `--path`
is omitted or relative.

## What it writes, and what it never writes

**Writes:** a new directory containing `pyproject.toml`, `README.md`,
`src/<package>/` (with `py.typed`) and `tests/`.

**Never writes:** a virtual environment, `uv.lock`, a `.python-version`, a Git
repository, or anything outside the new directory. It never replaces an
existing file, directory or symlink, and it never creates missing parent
directories.

## Errors you may hit

| Message | Fix |
| --- | --- |
| `could not create project directory …: File exists (os error 17)` | The target already exists. Pick another name or `--path`. |
| `target parent must be an existing directory: …` | Create the parent directory first. |
| `project name must contain ASCII letters, digits, dots, underscores, or hyphens, …` | Rename: remove slashes and spaces, avoid `CON`-style names and `-stubs`. |
| `invalid Python package name: json; use an ASCII identifier …` | Choose another `--package`, or another distribution name if the derived one collides. |
| `invalid command name: a b; use ASCII letters, digits, hyphens, or underscores …` | Fix `--command`. |
| `Python version must be MAJOR.MINOR, for example 3.12` | Use a value like `3.12`. |
| `the following required arguments were not provided: <NAME>` | Pass the project name. |

## Exit codes

| Code | Meaning |
| --- | --- |
| `0` | The project was created. |
| `1` | A validation or filesystem error (the messages above). |
| `2` | A command-line usage error, such as a missing name. |

## See also

- [`pyfaro sync`](sync.md): create `.venv` and `uv.lock` for the new project.
- [`pyfaro check`](check.md) and [`pyfaro test`](test.md): run the quality
  gate and the smoke test.
- [`pyfaro init`](init.md): adopt a project that already exists.
