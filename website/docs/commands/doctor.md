# pyfaro doctor

`pyfaro doctor` inspects a project and reports what is wrong with it, in one
screen: the project root, Pyfaro's configuration, Python, uv, Ruff, ty, pytest,
conflicting tool configuration, and whether the declared dependencies are
installed. Use it when a command fails for an unclear reason, when you adopt a
project, or as a first step in CI.

It is read-only in intent: it never installs, syncs or creates anything.

## Quick example

```console
$ pyfaro doctor
OK: project: ~/demo
OK: pyproject.toml: valid Pyfaro configuration
OK: Python: ~/.local/share/uv/python/cpython-3.13.11-linux-x86_64-gnu/bin/python3.13 (Python 3.13.11)
OK: uv: /home/you/.local/bin/uv (uv 0.9.26)
OK: ruff: ~/demo/.venv/bin/ruff (ruff 0.16.10)
OK: ty: ~/demo/.venv/bin/ty (ty 0.0.85)
OK: pytest: ~/demo/.venv/bin/pytest (pytest 9.1.1)
OK: dependencies: 3 runtime distribution(s) installed in .venv
OK: doctor: no blocking issues
```

```text
pyfaro doctor [--project <PATH>] [--uv <PATH>]
```

## How to read the report

Every line is `LEVEL: area: message`, in a fixed order, and a line is never
split (control characters in paths are escaped).

`OK`
:   The check passed.

`WARN`
:   Worth knowing, but not blocking. Warnings alone leave the exit code at `0`.

`ERROR`
:   Blocking. If there is at least one, doctor ends with
    `error: doctor found N blocking issue(s); …` and exits `1`.

The areas appear in this order: `project`, `pyproject.toml`, `configuration`
and `layout` findings (only when there is something to say), `Python`, `uv`,
`ruff`, `ty`, `pytest`, `dependencies`, and a final `doctor` line.

## Scenarios

### A healthy project

The quick example above. Before the first sync, the same project is not
healthy, and doctor says what to do:

```console
$ pyfaro doctor
OK: project: ~/bare
OK: pyproject.toml: valid Pyfaro configuration
OK: Python: /usr/bin/python3.14 (Python 3.14.8)
OK: uv: /home/you/.local/bin/uv (uv 0.9.26)
WARN: ruff: not found; install ruff in the project environment (.venv by default), activate an environment, or add it to PATH
WARN: ty: not found; install ty in the project environment (.venv by default), activate an environment, or add it to PATH
WARN: pytest: not found; install pytest in the project environment (.venv by default), activate an environment, or add it to PATH
ERROR: dependencies: project interpreter ~/bare/.venv/bin/python is missing; run pyfaro sync
error: doctor found 1 blocking issue(s); follow the actions above and run pyfaro doctor again
```

Run `pyfaro sync`, then doctor again.

### No project: `--project`

Outside a project, doctor still reports the global tools but marks the project
as an error:

```console
$ cd /tmp && pyfaro doctor
ERROR: project: no pyproject.toml found from /tmp; use --project <PATH>
OK: Python: /usr/bin/python3.14 (Python 3.14.8)
OK: uv: /home/you/.local/bin/uv (uv 0.9.26)
WARN: ruff: not found; …
error: doctor found 1 blocking issue(s); follow the actions above and run pyfaro doctor again
```

`--project <PATH>` selects exactly that directory instead of discovering the
nearest `pyproject.toml`.

### Choose uv: `--uv`

`--uv <PATH>` selects the uv executable. If it cannot be used, that is an
error, explained in one line with the next step:

```console
$ pyfaro doctor --project . --uv /nonexistent/uv
…
ERROR: uv: could not find uv at /nonexistent/uv; fix --uv <path> or install uv (https://docs.astral.sh/uv/getting-started/installation/, or `curl -LsSf https://astral.sh/uv/install.sh | sh`)
…
```

Without `--uv`, a uv that is not on `PATH` reads `could not find uv on PATH;
install uv (…) or pass --uv <path>`. On Windows the advice carries the
PowerShell installer command instead. Pyfaro never installs uv itself.

## Every kind of finding

### Tools (`ruff`, `ty`, `pytest`)

Found tools are `OK` with path and version. A missing or broken one is a
**warning** (the `WARN: ruff:` line above), because a project can work without
a given tool. Use [`pyfaro tools`](tools.md) for the details. Fix by running
`pyfaro add --dev <tool>` then `pyfaro sync`.

### Python

Python is found in the project environment, then `VIRTUAL_ENV`, then `PATH`.
A version outside `[project].requires-python` is an error:

```console
ERROR: Python: Python 3.13.11 does not satisfy requires-python ">=3.99"; select a compatible interpreter
```

A numeric `.python-version` pin (such as `3.12` or `3.12.4`) is compared with
the interpreter:

```console
ERROR: Python: Python 3.13.11 does not match .python-version "3.9"; select a matching interpreter or update the pin
```

A pin in a form doctor cannot compare is only a warning:

```console
WARN: Python: cannot compare .python-version request "bogus" with the selected interpreter; verify the pin
```

See [`pyfaro python`](python.md) to install and pin interpreters.

### uv

Same selection as every Pyfaro command; uv must be a stable `0.9.26` or newer.
A missing, too old or broken uv is an error, and each says what to do:

| Finding | Next step it names |
| --- | --- |
| `could not find uv on PATH` / `could not find uv at <path>` | Install uv, or fix `--uv <path>`. |
| `unsupported uv version: uv 0.9.25 …` | Run `uv self update`, reinstall uv, or select another with `--uv <path>`. |
| `uv --version failed for <path> (exit status: N): …` | Reinstall uv or select another with `--uv <path>`. |

### Project and Pyfaro configuration

`pyproject.toml` is validated with the loader used by `sync` and `run`:

```console
ERROR: configuration: invalid ~/d10/pyproject.toml: tool.pyfaro.schema-version must be the integer 1; correct pyproject.toml
```

```console
ERROR: configuration: [project].name must be a nonempty string; correct pyproject.toml
```

doctor also asks uv to validate `[tool.uv]` offline, which catches mistyped
settings. The message carries uv's own text, for example:

```console
ERROR: dependencies: uv rejected ~/d12/pyproject.toml: warning: Failed to parse `pyproject.toml` during settings discovery: … unknown field `not-a-setting`, expected one of …; correct the uv configuration
```

### Conflicting tool configuration

A tool must have one source of configuration. Both `[tool.ruff]` and
a `ruff.toml` next to it, or `[tool.pytest.ini_options]` and a `pytest.ini`,
is an error:

```console
ERROR: configuration: [tool.ruff] and ruff.toml both configure ruff; keep one source of configuration
ERROR: configuration: [tool.pytest] and pytest.ini both configure pytest; keep one source of configuration
```

The same applies to ty and to mixing native `[tool.pytest]` with
`[tool.pytest.ini_options]`:

```console
ERROR: configuration: [tool.pytest] and [tool.pytest.ini_options] both configure pytest; keep one format
```

### uv configuration

A project `uv.toml` takes precedence over `[tool.uv]` settings in
`pyproject.toml`. When both are present, that is a warning:

```console
WARN: configuration: uv.toml takes precedence over [tool.uv] configuration settings; project metadata remains active
```

`[tool.uv] managed = false` disables uv project management, so Pyfaro cannot
work with the project:

```console
ERROR: configuration: [tool.uv] managed = false disables uv project management; remove it before using pyfaro project workflows
```

### Workspaces

uv workspaces are not supported by Pyfaro, and doctor says so:

```console
ERROR: configuration: uv workspace at ~/d6/pyproject.toml is not supported by pyfaro doctor; use a standalone project
```

### Layout

If `src`, `tests` or the project environment is expected to be a directory but
is a file (or a dangling symlink), that is blocking:

```console
ERROR: layout: ~/d9/src must be a directory; move or remove the conflicting file
```

A project environment without its interpreter is also blocking (see the
`dependencies` example in the unsynced project above).

### Dependencies

Doctor reads `[project].dependencies` and the selected dependency groups (the
`dev` group by default), and checks that each applicable distribution is
installed in the project environment. It checks presence only, not versions,
extras or lockfile freshness; use `pyfaro sync --locked` for that. After
removing pytest from the environment:

```console
WARN: pytest: not found; install pytest in the project environment (.venv by default), activate an environment, or add it to PATH
ERROR: dependencies: pytest is missing from .venv; run pyfaro sync
```

A malformed requirement is an error that names the file to correct:

```console
ERROR: dependencies: invalid requirement "not a req!!"; correct ~/d13/pyproject.toml
```

A dependency group that is not selected by default is reported as an unchecked
warning (it is still validated for syntax):

```console
WARN: dependencies: group "docs" is not checked for installed distributions; sync it explicitly if needed
```

## Path handling

Doctor works from the project root. `--project` is a path relative to the
directory where you run `pyfaro`; everything else is reported with absolute
paths.

## Configuration

Doctor reads the `[tool.pyfaro]`, `[tool.uv]`, `[tool.ruff]`, `[tool.ty]` and
`[tool.pytest.ini_options]` sections and the standalone files that could
compete with them (`ruff.toml`, `.ruff.toml`, `ty.toml`, `pytest.ini`,
`uv.toml`, …). It does not edit any of them.

## Guarantees

- **No implicit sync.** Nothing is installed and no environment is created.
  Package indexes are never contacted.
- **No project writes.** Doctor does not change your files. Starting Python or
  a tool for a version probe can trigger that tool's own side effects, so a
  physically read-only filesystem is not guaranteed.

## Exit codes

| Code | Meaning |
| --- | --- |
| `0` | No `ERROR` (warnings are fine) |
| `1` | At least one `ERROR` |
| `130` | Cancelled |

A failed tool version probe is a finding, so a tool's own exit code is never
forwarded.

## See also

- [`pyfaro tools`](tools.md) — tool paths and versions.
- [`pyfaro sync`](sync.md) — fix a missing environment or dependencies.
- [`pyfaro python`](python.md) — fix interpreter mismatches.
