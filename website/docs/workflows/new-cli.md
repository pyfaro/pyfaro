# A new CLI

`create cli` generates an installable command-line application: a packaged
`src/` layout project with a console entry point and a test.

## 1. Create and sync

```console
$ pyfaro create cli greeter
Created CLI at ~/projects/greeter
$ cd greeter
$ pyfaro sync
Using CPython 3.13.11
Creating virtual environment at: .venv
Resolved 12 packages in 8ms
   Building greeter @ file://~/projects/greeter
      Built greeter @ file://~/projects/greeter
Prepared 1 package in 3ms
Installed 8 packages in 12ms
 + greeter==0.1.0 (from file://~/projects/greeter)
 + iniconfig==2.3.1
 + packaging==26.3
 + pluggy==1.7.0
 + pygments==2.21.0
 + pytest==9.1.1
 + ruff==0.16.10
 + ty==0.0.85
```

The command is declared in `pyproject.toml`:

```toml
[project.scripts]
greeter = "greeter.cli:main"
```

## 2. Run the command

[`pyfaro run`](../commands/run.md) executes a program from the project
environment, so the console script is on its path:

```console
$ pyfaro run -- greeter
Hello from greeter!
```

Everything after `--` belongs to the program, not to Pyfaro.

## 3. Make it do something

Replace `src/greeter/cli.py` with an `argparse` version:

```python
"""Console entry point for greeter."""

import argparse


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(prog="greeter", description="Greet someone.")
    parser.add_argument("name", nargs="?", default="world", help="who to greet")
    args = parser.parse_args(argv)
    print(f"Hello, {args.name}!")
    return 0
```

and update `tests/test_cli.py`:

```python
"""Tests for the greeter command."""

import pytest

from greeter.cli import main


def test_default(capsys: pytest.CaptureFixture[str]) -> None:
    assert main([]) == 0
    assert capsys.readouterr().out == "Hello, world!\n"


def test_name(capsys: pytest.CaptureFixture[str]) -> None:
    assert main(["Ada"]) == 0
    assert capsys.readouterr().out == "Hello, Ada!\n"
```

```console
$ pyfaro run -- greeter Ada
Hello, Ada!
$ pyfaro run -- greeter --help
usage: greeter [-h] [name]

Greet someone.

positional arguments:
  name        who to greet

options:
  -h, --help  show this help message and exit
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
rootdir: ~/projects/greeter
configfile: pyproject.toml
testpaths: tests
collected 2 items

tests/test_cli.py ..                                                     [100%]

============================== 2 passed in 0.01s ===============================
```

## 4. Distribute

Build the distributions with [`pyfaro build`](../commands/build.md), then
install the wheel as a tool:

```console
$ pyfaro build
Building source distribution (uv build backend)...
Building wheel from source distribution (uv build backend)...
Successfully built dist/greeter-0.1.0.tar.gz
Successfully built dist/greeter-0.1.0-py3-none-any.whl
built dist/greeter-0.1.0.tar.gz
built dist/greeter-0.1.0-py3-none-any.whl
$ uv tool install ./dist/greeter-0.1.0-py3-none-any.whl
Resolved 1 package in 0.99ms
Prepared 1 package in 2ms
Installed 1 package in 1ms
 + greeter==0.1.0 (from file://~/projects/greeter/dist/greeter-0.1.0-py3-none-any.whl)
Installed 1 executable: greeter
$ greeter Ada
Hello, Ada!
```

`greeter` is now on your `PATH`, outside the project environment. Remove it
with `uv tool uninstall greeter`. To share it, upload `dist/*` with
`uv publish`, after which anyone can run `uv tool install greeter`.

## Next steps

- [The quality gate](quality-gate.md) keeps the CLI tidy as it grows.
- [Continuous integration](continuous-integration.md) runs checks on every push.
- See [`create`](../commands/create.md) for naming rules and generated files.
