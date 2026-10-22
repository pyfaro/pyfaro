# Testing

`pyfaro test` runs [pytest](https://docs.pytest.org/) in the project
environment. Pyfaro does not reimplement pytest or interpret its configuration.
This command is available in published `0.3.0-alpha.1` and later versions.

```sh
pyfaro test [--project <PATH>] [PYTEST_ARGS]...
```

## Contract

- Runs `<project python> -m pytest <args>` with the project root as the working
  directory. Using the project environment's interpreter guarantees pytest
  imports that environment's packages. The interpreter is
  `.venv/bin/python` (`.venv\Scripts\python.exe` on Windows), or the
  environment named by `UV_PROJECT_ENVIRONMENT`.
- pytest reads the project's own configuration
  (`[tool.pytest.ini_options]`, `pytest.ini`, ...). Pyfaro passes no overrides.
- Output streams straight through and stdin is inherited, so it works in CI
  without a terminal. Cancellation stops the pytest process tree and exits 130.

## Project selection

Without `--project`, Pyfaro finds the nearest `pyproject.toml` from the current
directory upward. `--project <PATH>` selects exactly that directory.
Pyfaro's own options (`--project`) must come before the pytest arguments.

## Argument forwarding

Everything after Pyfaro's options is forwarded to pytest verbatim and in order:
filters, markers, node IDs and paths.

```sh
pyfaro test -k "parser and not slow" tests/
pyfaro test -m slow tests/test_io.py::test_read
pyfaro test --project ../app -- -x --maxfail=2
```

A leading `--` is accepted and removed; use it when the first pytest argument
could be confused with a Pyfaro option. Forwarded paths are relative to the
project root, not to the directory where you ran `pyfaro`, because pytest runs
there.

## Exit codes

pytest's exit status is returned unchanged: 0 all tests passed, 1 tests failed,
2 interrupted, 3 internal error, 4 usage error, 5 no tests collected.

When a delegated pytest run fails, Pyfaro adds a final stderr line
`error: pytest failed (exit status: N); see pytest's output above` and exits with
pytest's own status. pytest may write its own caches (`.pytest_cache` and `__pycache__`) in the project; Pyfaro
leaves source files untouched.

Pyfaro reports its own failures with exit 1 and an actionable message, so they
are never confused with test failures:

- The project environment interpreter does not exist: run `pyfaro sync`.
- pytest is not importable in the project environment: run
  `pyfaro add --dev pytest` then `pyfaro sync`.
- No project, or an invalid configuration.

Cancellation exits 130.

## No implicit sync

`pyfaro test` never creates or modifies `.venv` or `uv.lock`.

## New projects

`pyfaro create lib` and `pyfaro create cli` generate pytest-style tests, a
`dev` dependency group pinning pytest, and
`[tool.pytest.ini_options] testpaths = ["tests"]`. After `pyfaro sync`,
`pyfaro test` runs them with no further setup. An explicit selection that
matches nothing (for example `-k nomatch`) exits 5.

## Tested version

Tested with pytest 9.1.1. Other versions are not rejected.
