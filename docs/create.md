# Create a Python project

`pyfaro create` generates a small, packaged Python project. Creation writes
files locally; it does not run uv, install Python, resolve dependencies, or
download anything. The target's parent directory must already exist, and an
existing file, directory, or symlink is never replaced.

## Library

```sh
pyfaro create lib acme-widget --path ./projects/acme-widget --python 3.12
```

The distribution name is required. `--path` defaults to a directory with that
name under the current directory. `--python` defaults to `3.10` and sets the
minimum `requires-python` version; it does not choose or install an interpreter.
Use a name made of ASCII letters, digits, dots, underscores, and hyphens,
excluding reserved Windows device names such as `CON` and `COM1`. The
import package is the lowercased name with each run of dots, underscores, or
hyphens replaced by one underscore, so `acme-widget` creates
`src/acme_widget/`. If the distribution
name begins with a digit, the import package gets a leading underscore:
`1widget` creates `src/_1widget/`.

The template contains `pyproject.toml`, `README.md`, a typed package under
`src/`, and a `unittest` smoke test under `tests/`. It uses the uv build backend
and `[tool.pyfaro] schema-version = 1`.

## Command-line project

```sh
pyfaro create cli acme-tools --package acme_console --command acme-do \
  --path ./projects/acme-tools --python 3.12
```

`--package` selects the Python import package; by default it is derived from
the distribution name as above. It must be a Python identifier, not a Python
standard-library module, keyword, Python special name such as `__main__`, or
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

## Develop and build

From the generated directory, with a compatible Python interpreter already
installed:

```sh
pyfaro sync --offline
pyfaro run --locked --offline -- python -m unittest discover -s tests
uv --no-python-downloads --offline build
```

For a command-line project, also run `pyfaro run --locked --offline -- acme-do`
(substitute the selected command). `pyfaro sync` and uv create
`uv.lock` and `.venv` when needed. uv 0.9.26 is the tested version. Its bundled
build backend satisfies the generated `uv_build>=0.9.26,<0.10.0` requirement,
so these commands work offline for the dependency-free starter project. A
compatible installed Python is still required. The generated source has no
third-party runtime or test dependencies; adding dependencies later may
require an index or a populated uv cache.

Pyfaro's aggregate `check` command is planned for a later issue. The starter
source and tests use standard Python syntax and type annotations so they can
be checked by the future Ruff and ty workflow; `create` does not install or
invoke those tools.

The layout follows uv's [project templates](https://docs.astral.sh/uv/concepts/projects/init/)
and [build backend module settings](https://docs.astral.sh/uv/concepts/build-backend/).
