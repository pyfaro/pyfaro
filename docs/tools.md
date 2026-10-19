# Inspect external tools

`pyfaro tools` is available in the `0.1.0-alpha.2` development build. It
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
command, or unrecognized version produces an error and exit code 1. uv must be
a stable version >=0.9.26; CI tests 0.9.26. Ruff, ty, and pytest must identify
themselves with a parseable version, but this command does not claim a tested
compatibility range for them.

For Ruff, ty, and pytest, resolution checks the nearest project's `.venv`
first, then the environment named by `VIRTUAL_ENV`, then `PATH`. `--project`
selects an exact project directory with `pyproject.toml`. Without a project,
the activated environment and `PATH` are still checked. uv is resolved from
`PATH` unless `--uv` names an executable or path; it is validated using the
same minimum version rule as other Pyfaro commands. No dependencies are
installed, no environment is created or synchronized, and no shell is invoked.
Version probes use each tool's CLI: `--version` for uv, Ruff, and pytest, and
[`ty version`](https://docs.astral.sh/ty/reference/cli/) for ty.
