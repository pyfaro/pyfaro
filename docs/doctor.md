# Diagnose a project

`pyfaro doctor` inspects the nearest project without intentionally changing
project files. Use `--project <PATH>` to select an exact directory and
`--uv <PATH>` to select a specific uv executable.

```sh
pyfaro doctor
pyfaro doctor --project ./my-project --uv /path/to/uv
```

The report uses `OK`, `WARN`, and `ERROR` lines in a fixed order. It reports
the project root, Pyfaro configuration, Python, uv, Ruff, ty, pytest, and
selected dependencies. Paths and diagnostics escape control characters so each
finding stays on one line. Doctor exits 1 if it reports any `ERROR`; warnings
alone leave exit status 0. Cancellation exits 130. A failed tool version probe
is a doctor finding, so its child process exit code is not forwarded.

Python and uv are required. Python is searched in the project's `.venv`, then
`VIRTUAL_ENV`, then `PATH` (`python3` is a fallback on Unix). The reported
Python version is checked against `[project].requires-python`, when present.
Doctor also compares numeric `.python-version` pins (such as `3.12` or
`3.12.4`) with the selected interpreter. Other pin forms produce a warning
when doctor cannot compare them.

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

Doctor parses `[project].dependencies` and uv's default dependency groups
with PyPA's PEP 508 implementation available to the project interpreter
(including pip's bundled copy or the copy in CPython's `ensurepip` wheel).
The `dev` group is selected by default, unless `[tool.uv].default-groups`
or the project `uv.toml` changes that selection. Project `uv.toml` takes
precedence over `[tool.uv]`. Both table and inline-table group declarations
are accepted. Group names in defaults and includes match according
to PEP 735: case is ignored and runs of `-`, `_`, or `.` are equivalent.
Invalid group names, duplicate names after normalization, and explicitly
selected groups that are missing are blocking findings. Included groups are
followed. Legacy uv
`dev-dependencies` are also checked when `dev` is selected.
Non-default groups not included by a selected group are reported as unchecked
warnings, since `pyfaro sync` does not install them by default. Doctor still
validates their requirement strings, entry structure, and inclusion cycles,
without checking whether their distributions are installed. Invalid group
entries, inclusion cycles, and dynamic project dependencies are blocking
findings. An include table must contain only `include-group`.
Doctor evaluates environment markers and checks applicable distribution names
in the project's existing `.venv` using Python's standard
`importlib.metadata`. A missing project interpreter or distribution is a
blocking issue with a `pyfaro sync` action. If the parser is unavailable,
cannot run, or rejects any requirement, doctor reports a blocking finding
without an OK dependency summary. Doctor checks presence, not requirement
version constraints, optional extras, or lockfile freshness. Use
`pyfaro sync --locked` when a fresh, reproducible installation check is needed.

Doctor does not invoke a shell, access a package index, install packages, or
create a project environment. Its Python queries use isolated mode (`-I`),
disable bytecode writes (`-B`), and run from a temporary directory. Probe
stdout and stderr are captured in temporary files. Temporary files follow the
system temporary directory setting, which may point inside the project.
Starting Python or another tool for a version probe can also trigger that
tool's own startup side effects, so doctor cannot guarantee a physically
read-only filesystem.
