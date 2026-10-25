---
description: "Install, list and pin Python versions with pyfaro python, which delegates interpreter management to uv."
---

# Python versions

Pyfaro delegates interpreter management to uv. `pyfaro python` is a thin
wrapper with three subcommands: `list`, `install` and `pin`. Pyfaro itself
never downloads Python; only `python install` does, and only when you ask.

## See what is available

```console
$ pyfaro python list
cpython-3.15.0a5-linux-x86_64-gnu                 <download available>
cpython-3.14.8-linux-x86_64-gnu                   /usr/bin/python3.14
cpython-3.13.11-linux-x86_64-gnu                  /home/you/.local/share/uv/python/cpython-3.13.11-linux-x86_64-gnu/bin/python3.13
cpython-3.12.12-linux-x86_64-gnu                  /home/you/.local/share/uv/python/cpython-3.12.12-linux-x86_64-gnu/bin/python3.12
cpython-3.11.14-linux-x86_64-gnu                  /home/you/.local/share/uv/python/cpython-3.11.14-linux-x86_64-gnu/bin/python3.11
cpython-3.10.19-linux-x86_64-gnu                  <download available>
...
```

Installed interpreters show their path; the rest show
`<download available>`. The output is trimmed here and differs per machine.

## Install a version

```console
$ pyfaro python install 3.12
Python 3.12 is already installed
```

If it is missing, uv downloads it. With `--offline`, nothing is downloaded and
the command fails instead:

```console
$ pyfaro python --offline install 3.14
error: Failed to install cpython-3.14.2-linux-x86_64-gnu
  Caused by: Failed to download https://github.com/astral-sh/python-build-standalone/releases/download/20260114/cpython-3.14.2%2B20260114-x86_64-unknown-linux-gnu-install_only_stripped.tar.gz
  Caused by: Network connectivity is disabled, but the requested data wasn't found in the cache for: `https://github.com/astral-sh/python-build-standalone/releases/download/20260114/cpython-3.14.2%2B20260114-x86_64-unknown-linux-gnu-install_only_stripped.tar.gz`
error: uv python install failed (exit status: 1); see uv's output above
```

Note that `--offline` goes before the subcommand.

## Pin the project's version

`pin` writes `.python-version` next to the project's `pyproject.toml`, and
without an argument it shows the current pin:

```console
$ pyfaro python pin 3.12
Pinned `.python-version` to `3.12`
$ pyfaro python pin
3.12
```

Sync again to rebuild `.venv` with the pinned interpreter:

```console
$ pyfaro sync
Using CPython 3.12.12
Removed virtual environment at: .venv
Creating virtual environment at: .venv
Resolved 13 packages in 0.57ms
Installed 9 packages in 16ms
 + humanize==4.16.0
 + iniconfig==2.3.1
 + packaging==26.3
 + pluggy==1.7.0
 + pygments==2.21.0
 + pytest==9.1.1
 + ruff==0.16.10
 + textkit==0.1.0 (from file://~/projects/textkit)
 + ty==0.0.85
$ pyfaro run -- python --version
Python 3.12.12
```

Commit `.python-version` so your teammates and CI use the same interpreter.

## Keep `requires-python` consistent

`requires-python` in `pyproject.toml` declares which interpreters the project
supports, and the pin must satisfy it. If the two disagree, uv refuses. Here
the project requires Python `>=3.13` but the pin says 3.12:

```console
$ pyfaro python pin 3.12
error: The requested Python version `3.12` is incompatible with the project `requires-python` value of `>=3.13`.
error: uv python pin failed (exit status: 2); see uv's output above
```

If the pin was already written (for example before you raised
`requires-python`), `sync` and `doctor` both catch it:

```console
$ pyfaro sync
Using CPython 3.12.12
error: The Python request from `.python-version` resolved to Python 3.12.12, which is incompatible with the project's Python requirement: `>=3.13` (from `project.requires-python`)
Use `uv python pin` to update the `.python-version` file to a compatible version
error: uv sync failed (exit status: 2); see uv's output above
$ pyfaro doctor
OK: project: ~/projects/textkit
OK: pyproject.toml: valid Pyfaro configuration
OK: Python: ~/.local/share/uv/python/cpython-3.12.12-linux-x86_64-gnu/bin/python3.12 (Python 3.12.12)
ERROR: Python: Python 3.12.12 does not satisfy requires-python ">=3.13"; select a compatible interpreter
OK: uv: /home/you/.local/bin/uv (uv 0.9.26)
OK: ruff: ~/projects/textkit/.venv/bin/ruff (ruff 0.16.10)
OK: ty: ~/projects/textkit/.venv/bin/ty (ty 0.0.85)
OK: pytest: ~/projects/textkit/.venv/bin/pytest (pytest 9.1.1)
OK: dependencies: 4 runtime distribution(s) installed in .venv
error: doctor found 1 blocking issue(s); follow the actions above and run pyfaro doctor again
```

Fix it by pinning a compatible version and syncing:

```console
$ pyfaro python pin 3.13
Updated `.python-version` from `3.12` -> `3.13`
$ pyfaro sync
Using CPython 3.13.11
Removed virtual environment at: .venv
Creating virtual environment at: .venv
...
```

!!! tip "Pick the lower bound for development"
    Pinning the oldest version your `requires-python` allows catches
    accidental use of newer syntax, while a matrix in
    [CI](continuous-integration.md#several-python-versions) covers the rest.

## Next steps

- [`python` command reference](../commands/python.md)
- [Troubleshooting](troubleshooting.md) for other `doctor` findings.
