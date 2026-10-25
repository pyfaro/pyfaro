---
description: "Run the test suite with pytest through pyfaro test, inside the project environment."
---

# pyfaro test

`pyfaro test` runs [pytest](https://docs.pytest.org/) with the project
environment's own Python, so your tests import the packages installed there.
Use it for the day-to-day "run the tests" loop and in CI.

Pyfaro does not reimplement pytest or interpret its configuration.

## Quick example

```console
$ pyfaro test
============================= test session starts ==============================
platform linux -- Python 3.13.11, pytest-9.1.1, pluggy-1.7.0
rootdir: ~/demo
configfile: pyproject.toml
testpaths: tests
collected 1 item

tests/test_library.py .                                                  [100%]

============================== 1 passed in 0.01s ===============================
```

```text
pyfaro test [--project <PATH>] [PYTEST_ARGS]...
```

It runs `<project python> -m pytest <args>` from the project root. In a
project made with `pyfaro create lib` or `cli` and then `pyfaro sync`, tests
and pytest are already set up.

## Forwarding arguments

Everything after Pyfaro's own options goes to pytest verbatim and in order:
filters, markers, node IDs, paths and any pytest flag. The scenarios below add
`tests/test_parser.py` with two ordinary tests and one marked `slow`.

### Filter by keyword: `-k`

```console
$ pyfaro test -k "parse and not slow"
============================= test session starts ==============================
platform linux -- Python 3.13.11, pytest-9.1.1, pluggy-1.7.0
rootdir: ~/t
configfile: pyproject.toml
testpaths: tests
collected 4 items / 2 deselected / 2 selected

tests/test_parser.py ..                                                  [100%]

======================= 2 passed, 2 deselected in 0.01s ========================
```

Quote expressions as you would for pytest, so your shell passes them as one
argument.

### Filter by marker: `-m`

```console
$ pyfaro test -m slow
…
collected 4 items / 3 deselected / 1 selected

tests/test_parser.py .                                                   [100%]

======================= 1 passed, 3 deselected in 0.01s ========================
```

Register markers under pytest's own configuration (`markers = ["slow: …"]`)
so pytest does not warn about them.

### Run one test by node ID, or several paths

```console
$ pyfaro test tests/test_parser.py::test_parse_ok
…
collected 1 item

tests/test_parser.py .                                                   [100%]

============================== 1 passed in 0.01s ===============================
```

```console
$ pyfaro test -v tests/test_parser.py::test_parse_empty tests/test_library.py
…
tests/test_parser.py::test_parse_empty PASSED                            [ 50%]
tests/test_library.py::test_hello PASSED                                 [100%]

============================== 2 passed in 0.01s ===============================
```

Explicit paths replace `testpaths`, which is why the header line
`testpaths: tests` disappears.

### Any other pytest flag

```console
$ pyfaro test -q --co
tests/test_library.py::test_hello
tests/test_parser.py::test_parse_ok
tests/test_parser.py::test_parse_empty
tests/test_parser.py::test_parse_slow

4 tests collected in 0.00s
```

### Pyfaro's `--verbose` goes before `test`

`-v` and `--verbose` after `test` belong to pytest, as in the example above.
Pyfaro's own trace of the pytest version, path and command line is requested
before the subcommand, where it leaves pytest quiet:

```console
$ pyfaro -v test
pyfaro test: running ~/demo/.venv/bin/pytest --version
pyfaro test: using pytest 9.1.1 at ~/demo/.venv/bin/pytest
pyfaro test: running ~/demo/.venv/bin/python -c "import importlib.util, sys; sys.exit(importlib.util.find_spec('pytest') is None)"
pyfaro test: running ~/demo/.venv/bin/python -m pytest
```

### Separate with `--`

A leading `--` is accepted and removed. Use it when the first pytest argument
could be mistaken for a Pyfaro option:

```console
$ pyfaro test -- -x -q
....                                                                     [100%]
4 passed in 0.00s
```

### Choose the project: `--project`

`--project <PATH>` selects exactly that directory instead of the nearest
`pyproject.toml` above the current directory. It must come **before** the
pytest arguments; after them it is forwarded to pytest, which rejects it:

```console
$ pyfaro test tests -q --project t2
ERROR: usage: python -m pytest [options] [file_or_dir] [file_or_dir] [...]
python -m pytest: error: unrecognized arguments: --project
  inifile: ~/t2/pyproject.toml
  rootdir: ~/t2

error: pytest failed (exit status: 4); see pytest's output above
```

Put it first: `pyfaro test --project t2 tests -q`.

## Path handling

This differs from `lint`, `fmt`, `type` and `check`:

!!! warning "Test paths are relative to the project root"
    pytest runs with the **project root** as its working directory, and
    Pyfaro forwards your paths unchanged. Paths are therefore relative to the
    project root, **not** to the directory where you ran `pyfaro`.

From inside `tests/`, `test_parser.py` is not found, but the project-relative
path works:

```console
$ cd tests
$ pyfaro test test_parser.py
ERROR: file or directory not found: test_parser.py

============================= test session starts ==============================
…
collected 0 items

============================ no tests ran in 0.01s =============================
error: pytest failed (exit status: 4); see pytest's output above
$ pyfaro test tests/test_parser.py -q
...                                                                      [100%]
3 passed in 0.00s
```

## Configuration

pytest reads its own configuration. A generated project contains:

```toml
[tool.pytest.ini_options]
testpaths = ["tests"]
```

Add options in the same table, for example `markers`, `addopts` or
`norecursedirs`. To keep a directory out of collection by default:

```toml
[tool.pytest.ini_options]
testpaths = ["tests"]
norecursedirs = ["tests/slow_fixtures"]
```

Keep one source of pytest configuration. `pyfaro doctor` reports
`[tool.pytest.ini_options]` next to a `pytest.ini`, and native `[tool.pytest]`
settings with pytest older than 9. Pyfaro passes no overrides.

## Guarantees

- **No implicit sync.** `.venv` and `uv.lock` are never created or modified.
- **No source writes.** Pyfaro does not edit files. pytest and Python write
  their usual caches (`.pytest_cache`, `__pycache__`).
- Output streams through, stdin is inherited (it works in CI without a
  terminal), and cancelling stops pytest's process tree.

## Errors and fixes

pytest's exit status is returned unchanged, so your failing test is never
confused with a Pyfaro problem. Pyfaro's own failures exit `1` with an
actionable message:

```console
$ pyfaro test
error: pytest: project environment interpreter not found at ~/bare/.venv/bin/python; run `pyfaro sync` first
```

Run `pyfaro sync`. If the environment exists but lacks pytest:

```console
$ pyfaro test
error: pytest: not installed in the project environment (~/t3/.venv/bin/python); run `pyfaro add --dev pytest` then `pyfaro sync`
```

Outside a project: `error: no pyproject.toml found from …; use --project <PATH>`.

## Exit codes

| Code | Meaning | Example |
| --- | --- | --- |
| `0` | All tests passed | |
| `1` | Tests failed | `1 failed, 4 passed` |
| `2` | Interrupted | an `ImportError` while collecting a test file |
| `3` | Internal error | an exception raised inside a pytest hook (`INTERNALERROR>`) |
| `4` | Usage error | an unknown flag, or a path that does not exist |
| `5` | No tests collected | `-k nomatch` |

A failing run looks like this:

```console
$ pyfaro test -q
F....                                                                    [100%]
=================================== FAILURES ===================================
__________________________________ test_fails __________________________________

    def test_fails() -> None:
>       assert 1 + 1 == 3
E       assert (1 + 1) == 3

tests/test_broken.py:2: AssertionError
=========================== short test summary info ============================
FAILED tests/test_broken.py::test_fails - assert (1 + 1) == 3
1 failed, 4 passed in 0.02s
error: pytest failed (exit status: 1); see pytest's output above
```

A selection that matches nothing exits `5`, which CI treats as a failure:

```console
$ pyfaro test -k nomatch
…
collected 4 items / 4 deselected / 0 selected

============================ 4 deselected in 0.01s =============================
error: pytest failed (exit status: 5); see pytest's output above
```

Pyfaro failures (no project, missing interpreter or pytest) also exit `1`, and
cancellation exits `130`. Tell them apart by the message: Pyfaro's start with
`error:` before pytest ran.

## See also

- [`pyfaro check`](check.md) — format, lint and type check (tests are separate).
- [`pyfaro sync`](sync.md) — create the environment pytest runs in.
- [`pyfaro add`](add-remove.md) — add pytest to your `dev` group.
