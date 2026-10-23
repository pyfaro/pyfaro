# First steps

This tour takes about five minutes. You create a library, install its
environment, add a dependency, run code, and use the quality gate, the test
runner and the doctor. It assumes [Pyfaro and uv are installed](installation.md)
and that uv can find a Python interpreter.

The outputs below come from a real run on Linux with uv 0.9.26. Paths are
shortened and uv's hardlink warning is omitted.

## 1. Create a project

```console
$ pyfaro create lib greeter
Created library at ~/code/greeter
$ cd greeter
$ ls -A
pyproject.toml  README.md  src  tests
```

`create` writes a src-layout package, a smoke test and a `pyproject.toml`
without calling uv. The generated file already carries everything the other
commands need:

```toml
[project]
name = "greeter"
version = "0.1.0"
readme = "README.md"
requires-python = ">=3.10"
dependencies = []

[build-system]
requires = ["uv_build>=0.9.26,<0.10.0"]
build-backend = "uv_build"

[dependency-groups]
dev = ["pytest==9.1.1", "ruff==0.16.10", "ty==0.0.85"]

[tool.pyfaro]
schema-version = 1
```

Use `pyfaro create cli` for an installable command-line application. See
[`pyfaro create`](../commands/create.md).

## 2. Sync the environment

```console
$ pyfaro sync
Using CPython 3.13.11
Creating virtual environment at: .venv
Resolved 12 packages in 19ms
   Building greeter @ file:///~/code/greeter
      Built greeter @ file:///~/code/greeter
Prepared 1 package in 7ms
Installed 8 packages in 168ms
 + greeter==0.1.0 (from file:///~/code/greeter)
 + iniconfig==2.3.1
 + packaging==26.3
 + pluggy==1.7.0
 + pygments==2.21.0
 + pytest==9.1.1
 + ruff==0.16.10
 + ty==0.0.85
```

That is uv's own output: Pyfaro runs `uv sync` and streams the result. It
created `uv.lock` and `.venv`. See [`pyfaro sync`](../commands/sync.md).

## 3. Add a dependency

```console
$ pyfaro add httpx
Resolved 18 packages in 16ms
$ pyfaro add --dev rich
Resolved 21 packages in 22ms
```

`add` edits `pyproject.toml` and `uv.lock` through uv. It does not install
anything: the next `sync` or `run` brings the environment up to date.

## 4. Run something

```console
$ pyfaro run -- python -c "import httpx; print(httpx.__version__)"
Installed 11 packages in 45ms
0.28.1
```

`run` executes the command in the project environment, installing the pending
packages first. Everything after `--` is passed through unchanged. See
[`pyfaro run`](../commands/run.md).

## 5. Check the code

```console
$ pyfaro check
3 files already formatted
All checks passed!
All checks passed!

== pyfaro check summary ==
format: ok
lint: ok
type: ok
check: ok
```

`check` runs the format check, Ruff lint and the ty type check, and prints a
summary. It never writes files unless you pass `--fix`. When a step fails,
the summary says which one and the command exits with code 1:

```console
$ pyfaro check
...
== pyfaro check summary ==
format: failed (exit 1)
lint: failed (exit 1)
type: failed (exit 1)
check: failed (format, lint, type)
$ echo $?
1
```

Run `pyfaro check --fix` to apply Ruff's safe fixes and formatting, then
verify again. See [`pyfaro check`](../commands/check.md).

## 6. Run the tests

```console
$ pyfaro test
============================= test session starts ==============================
platform linux -- Python 3.13.11, pytest-9.1.1, pluggy-1.7.0
rootdir: ~/code/greeter
configfile: pyproject.toml
testpaths: tests
collected 1 item

tests/test_library.py .                                                  [100%]

============================== 1 passed in 0.01s ===============================
```

Arguments are forwarded to pytest, and so is its exit code. For example
`pyfaro test -k parser` runs only the matching tests. See
[`pyfaro test`](../commands/test.md).

## 7. Ask the doctor

```console
$ pyfaro doctor
OK: project: ~/code/greeter
OK: pyproject.toml: valid Pyfaro configuration
OK: Python: ~/.local/share/uv/python/cpython-3.13.11-linux-x86_64-gnu/bin/python3.13 (Python 3.13.11)
OK: uv: ~/.local/bin/uv (uv 0.9.26)
OK: ruff: ~/code/greeter/.venv/bin/ruff (ruff 0.16.10)
OK: ty: ~/code/greeter/.venv/bin/ty (ty 0.0.85)
OK: pytest: ~/code/greeter/.venv/bin/pytest (pytest 9.1.1)
OK: dependencies: 5 runtime distribution(s) installed in .venv
OK: doctor: no blocking issues
```

`doctor` is a read-only report. It exits 1 if any line is an `ERROR`; `WARN`
lines alone leave the exit code at 0. Use it first whenever something looks
wrong. See [`pyfaro doctor`](../commands/doctor.md).

!!! tip "Where did the tools come from?"
    Ruff, ty and pytest come from the project's `dev` dependency group, so
    `sync` installs them in `.venv`. Pyfaro finds them there first. Read
    [Environments and tools](../concepts/environments-and-tools.md).

## Next steps

- [Features](features.md): every command at a glance.
- [Workflows](../workflows/index.md): a [new library](../workflows/new-library.md),
  [a FastAPI app](../workflows/fastapi.md),
  [the quality gate](../workflows/quality-gate.md) and
  [continuous integration](../workflows/continuous-integration.md).
- [How it works](../concepts/how-it-works.md).
