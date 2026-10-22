# Inspect external tools

`pyfaro tools` is available in published `0.1.0-alpha.2` and later versions. It
prints one line each for uv, Ruff, ty, and pytest, with the resolved executable
path and reported version. Unavailable or incompatible tools appear as
`unavailable` with a reason. The list is informational and exits successfully.

```sh
pyfaro tools
pyfaro tools ruff
pyfaro tools pytest --project ./my-project
pyfaro tools uv --uv /path/to/uv
```

Select a name to require that tool: a missing executable, failed version
command, or unrecognized version produces an error and exit code 1. Failed
probes include the selected path and up to 4096 bytes of standard error.
Control characters in paths and diagnostics are escaped to keep listings and
strict-check errors on one line. A strict tools check returns 1 even if its
version probe exits with another code; delegated uv workflow exit codes are
unchanged. uv must be a stable version >=0.9.26; CI tests 0.9.26.
Ruff, ty, and pytest must identify
themselves with a parseable version; pytest is checked using its
[PEP 440](https://packaging.python.org/en/latest/specifications/version-specifiers/)
version scheme, including prereleases. CI tests Ruff 0.16.10, ty 0.0.85, and
pytest 9.1.1; these versions are documented, not enforced, so other versions
that identify themselves are accepted.

For Ruff, ty, and pytest, resolution checks the nearest project's environment
first, then the environment named by `VIRTUAL_ENV`, then `PATH`. By default the
project environment is `.venv`; `UV_PROJECT_ENVIRONMENT` can select another
absolute path or a path relative to the project root. `--project`
selects an exact project directory with `pyproject.toml`. Without a project,
the activated environment and `PATH` are still checked. uv is resolved from
`PATH` unless `--uv` names an executable or path; it is validated using the
same minimum version rule as other Pyfaro commands. No dependencies are
installed, no environment is created or synchronized, and no shell is invoked.
Version probes use each tool's CLI: `--version` for uv, Ruff, and pytest, and
[`ty version`](https://docs.astral.sh/ty/reference/cli/) for ty.
