# A new library

This recipe takes a Python library from an empty directory to built
distributions: create, sync, add a dependency, write code, check, test and
build.

## 1. Create the project

```console
$ pyfaro create lib textkit
Created library at ~/projects/textkit
$ cd textkit
```

`create` writes a `src/` layout package, a smoke test and a `pyproject.toml`
with the development tools pinned in the `dev` dependency group. It does not
run uv or download anything.

```text
textkit/
├── README.md
├── pyproject.toml
├── src/textkit/__init__.py
├── src/textkit/py.typed
└── tests/test_library.py
```

## 2. Sync the environment

```console
$ pyfaro sync
Using CPython 3.13.11
Creating virtual environment at: .venv
Resolved 12 packages in 93ms
   Building textkit @ file://~/projects/textkit
      Built textkit @ file://~/projects/textkit
Prepared 1 package in 4ms
Installed 8 packages in 13ms
 + iniconfig==2.3.1
 + packaging==26.3
 + pluggy==1.7.0
 + pygments==2.21.0
 + pytest==9.1.1
 + ruff==0.16.10
 + textkit==0.1.0 (from file://~/projects/textkit)
 + ty==0.0.85
```

This creates `.venv` and `uv.lock`. Commit `uv.lock`; CI uses it with
`sync --locked`.

## 3. Add a dependency

```console
$ pyfaro add "humanize>=4"
Resolved 13 packages in 9ms
$ pyfaro sync
Resolved 13 packages in 0.65ms
   Building textkit @ file://~/projects/textkit
      Built textkit @ file://~/projects/textkit
Prepared 2 packages in 152ms
Uninstalled 1 package in 0.36ms
Installed 2 packages in 3ms
 + humanize==4.16.0
 ~ textkit==0.1.0 (from file://~/projects/textkit)
```

`add` edits `pyproject.toml` and `uv.lock`; it does not touch `.venv`. Run
`sync` afterwards to install.

!!! warning "Forgetting the sync"
    Until you sync, imports of the new package fail in `check` (ty cannot
    resolve it) and in `test` (`ModuleNotFoundError`).

## 4. Write some code

Edit `src/textkit/__init__.py`:

```python
"""Public API for textkit."""

import humanize


def hello() -> str:
    return "Hello from textkit!"


def size(num_bytes: int) -> str:
    """Return a human-readable size, for example ``1.5 MB``."""
    return humanize.naturalsize(num_bytes)
```

and add `tests/test_size.py`:

```python
from textkit import size


def test_size() -> None:
    assert size(1_500_000) == "1.5 MB"
```

## 5. Check and test

```console
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
platform linux -- Python 3.13.11, pytest-9.1.1, pluggy-1.7.0
rootdir: ~/projects/textkit
configfile: pyproject.toml
testpaths: tests
collected 2 items

tests/test_library.py .                                                  [ 50%]
tests/test_size.py .                                                     [100%]

============================== 2 passed in 0.02s ===============================
```

[`check`](../commands/check.md) runs the format check, Ruff and ty;
[`test`](../commands/test.md) runs pytest and returns its exit code.

## 6. Build

Pyfaro does not have a `build` command yet (it is planned). Build with uv
directly, which the generated `pyproject.toml` is already set up for:

```console
$ uv build
Building source distribution (uv build backend)...
Building wheel from source distribution (uv build backend)...
Successfully built dist/textkit-0.1.0.tar.gz
Successfully built dist/textkit-0.1.0-py3-none-any.whl
```

The wheel and source distribution are in `dist/`. Publish them with
`uv publish` when you are ready.

## Next steps

- Gate every commit: [The quality gate](quality-gate.md).
- Run the same steps on every push: [Continuous integration](continuous-integration.md).
- Support several interpreters: [Python versions](python-versions.md).
