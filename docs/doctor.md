# Diagnose a project

`pyfaro doctor` inspects the nearest project without changing it. Use
`--project <PATH>` to select an exact directory and `--uv <PATH>` to select a
specific uv executable.

```sh
pyfaro doctor
pyfaro doctor --project ./my-project --uv /path/to/uv
```

The report uses `OK`, `WARN`, and `ERROR` lines in a fixed order. It reports
the project root, Pyfaro configuration, Python, uv, Ruff, ty, pytest, and
runtime dependencies. Paths and diagnostics escape control characters so each
finding stays on one line. Doctor exits 1 if it reports any `ERROR`; warnings
alone leave exit status 0. Cancellation exits 130. A failed tool version probe
is a doctor finding, so its child process exit code is not forwarded.

Python and uv are required. Python is searched in the project's `.venv`, then
`VIRTUAL_ENV`, then `PATH` (`python3` is a fallback on Unix). The reported
Python version is checked against `[project].requires-python`, when present.
uv follows the normal Pyfaro executable selection and requires stable uv
0.9.26 or newer. Ruff, ty, and pytest use the same discovery and version rules
as `pyfaro tools`; missing or broken optional tools are warnings. To fix a
missing executable, install it in the intended environment or put it on
`PATH`, then run doctor again.

Doctor validates `pyproject.toml` with the same Pyfaro config loader used by
`sync` and `run`. It reports conflicting per-tool files alongside
`[tool.ruff]`, `[tool.ty]`, `[tool.pytest]`, or `[tool.uv]`, and detects files
where `src`, `tests`, or `.venv` directories are expected. A `.venv` directory
without its Python interpreter is a blocking layout issue. Remove the
conflicting file or keep one source of configuration as the report directs.

For unconditional `[project].dependencies`, doctor checks distribution names
in the project's existing `.venv` using Python's standard
`importlib.metadata`. A missing project interpreter or distribution is a
blocking issue with a `pyfaro sync` action. Conditional requirements are
reported as warnings because their markers depend on the target environment.
Doctor checks presence, not requirement version constraints, optional
dependency groups, or lockfile freshness. Use `pyfaro sync --locked` when a
fresh, reproducible installation check is needed.

Doctor does not invoke a shell, install packages, create an environment, or
write project files. Its probes use executable version commands and a
read-only Python metadata query.
