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
`[tool.ruff]`, `[tool.ty]`, or `[tool.pytest]`, and detects files where `src`,
`tests`, or `.venv` directories are expected. A `.venv` directory without its
Python interpreter is a blocking layout issue. On Unix, an existing
`.venv/bin/python` without executable permission also blocks, even if another
Python is available on `PATH` and the project has no dependencies. For pytest,
`pytest.ini`, `.pytest.ini`, `pytest.toml`, and `.pytest.toml` conflict with
`[tool.pytest.ini_options]` or native `[tool.pytest]` settings. The TOML files
are supported by pytest 9. `tox.ini` counts only when it has `[pytest]`, and
`setup.cfg` counts only when it has `[tool:pytest]`. Unrelated nested
`[tool.pytest]` tables do not count as pytest configuration. Multiple pytest
configuration files also conflict. For uv, a project `uv.toml`
takes precedence over `[tool.uv]` in `pyproject.toml`; doctor reports their
coexistence as a warning. A non-table `[tool.uv]` value is an error only when
that pyproject configuration is selected.

Doctor parses `[project].dependencies` and uv's default dependency groups
with PyPA's PEP 508 implementation available to the project interpreter
(including pip's bundled copy or the copy in CPython's `ensurepip` wheel).
The `dev` group is selected by default, unless uv's `default-groups` setting
changes that selection. `UV_NO_DEV`, `UV_NO_DEFAULT_GROUPS`, and the
space-delimited `UV_NO_GROUP` environment variable remove groups from installed
distribution checks, as they do for uv. Doctor discovers the nearest project
configuration from the project directory upward: in each directory, `uv.toml`
takes precedence over `[tool.uv]` in `pyproject.toml`. It also reads user
configuration from `$XDG_CONFIG_HOME/uv/uv.toml` (defaulting to
`~/.config/uv/uv.toml`) on Unix or `%APPDATA%\uv\uv.toml` on Windows, and
system configuration from the first available `$XDG_CONFIG_DIRS/uv/uv.toml`
(defaulting to `/etc/xdg/uv/uv.toml`) or `/etc/uv/uv.toml` on Unix, or
`%PROGRAMDATA%\uv\uv.toml` on Windows. Project settings take precedence over
user settings, which take precedence over system settings; array settings,
including `default-groups` and `dev-dependencies`, concatenate in that order
when both layers provide arrays. For a scalar/array mismatch, the
higher-priority setting wins.
`UV_CONFIG_FILE` selects an explicit standalone uv configuration file instead
of discovery; relative paths are resolved from the selected project directory,
as they are for uv commands run by Pyfaro. `UV_NO_CONFIG` disables discovery
unless `UV_CONFIG_FILE` is also set. Missing explicit files and unreadable or
malformed discovered files are blocking findings that identify the file to fix.
Both table and inline-table group declarations are accepted. Group names in
defaults and includes match according to PEP 735: case is ignored and runs of
`-`, `_`, or `.` are equivalent.
Invalid group names, duplicate names after normalization, and explicitly
selected groups that are missing are blocking findings. Included groups are
followed. Legacy uv `dev-dependencies` are validated even when `dev` is not
selected. Their installed distributions are checked only when `dev` is
selected, including when `dev` is explicitly listed in `default-groups` and
legacy `dev-dependencies` are its only definition. Malformed requirements
identify the uv configuration file to correct. Non-default groups not included
by a selected group are reported as unchecked warnings. Doctor still validates
every group's requirement strings, entry structure, and inclusion cycles
without checking whether their distributions are installed. Invalid group
entries, inclusion cycles, and dynamic project dependencies are blocking
findings, even when a static dependencies array is also present. Explicit
default group names are validated even when environment switches exclude them.
An include table must contain only `include-group`.
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
