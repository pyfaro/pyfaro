---
description: "See which uv, Ruff, ty and pytest executables Pyfaro uses, and where they come from, with pyfaro tools."
---

# pyfaro tools

`pyfaro tools` shows which executables Pyfaro would use for **uv**, **Ruff**,
**ty** and **pytest**, and the version each one reports. Use it when a command
says a tool is missing, or to check that CI picks up the same binaries you use
locally.

It is informational by default and never installs, syncs or creates anything.

## Quick example

```console
$ pyfaro tools
uv: /home/you/.local/bin/uv (uv 0.9.26)
ruff: ~/demo/.venv/bin/ruff (ruff 0.16.10)
ty: ~/demo/.venv/bin/ty (ty 0.0.85)
pytest: ~/demo/.venv/bin/pytest (pytest 9.1.1)
```

One line per tool, always in the order uv, ruff, ty, pytest. This output comes
from a project made with `pyfaro create lib demo` and then `pyfaro sync`.

```text
pyfaro tools [TOOL] [--project <PATH>] [--uv <PATH>]
```

## Scenarios

### List every tool

`pyfaro tools` with no argument is a listing. A tool that cannot be found is
printed as `unavailable` with a reason, and the command still exits `0`. Here
is the same project before its first `pyfaro sync`, with no tool installed
anywhere on `PATH`:

```console
$ pyfaro tools
uv: /home/you/.local/bin/uv (uv 0.9.26)
ruff: unavailable (ruff: not found; install ruff in the project environment (.venv by default), activate an environment, or add it to PATH)
ty: unavailable (ty: not found; install ty in the project environment (.venv by default), activate an environment, or add it to PATH)
pytest: unavailable (pytest: not found; install pytest in the project environment (.venv by default), activate an environment, or add it to PATH)
```

### Check one tool strictly

Name a tool (`uv`, `ruff`, `ty` or `pytest`) to turn the listing into a
check. The command prints that single line when the tool works, and **fails
with exit code 1** when the executable is missing, its version command fails,
or its version cannot be parsed.

```console
$ pyfaro tools ruff
ruff: ~/demo/.venv/bin/ruff (ruff 0.16.10)
$ echo $?
0
```

```console
$ pyfaro tools ruff
error: ruff: not found; install ruff in the project environment (.venv by default), activate an environment, or add it to PATH
$ echo $?
1
```

This is the form to use in scripts: `pyfaro tools ruff && pyfaro tools ty`.

An unknown name is rejected by the argument parser with exit code `2`:

```console
$ pyfaro tools nope
error: invalid value 'nope' for '[TOOL]'
  [possible values: uv, ruff, ty, pytest]

For more information, try '--help'.
```

### Choose the project: `--project`

By default Pyfaro looks for the nearest `pyproject.toml` from the current
directory upward. `--project <PATH>` selects exactly that directory instead,
so you can inspect a project from anywhere.

```console
$ pyfaro tools ruff --project ~/bare
error: ruff: not found; install ruff in the project environment (.venv by default), activate an environment, or add it to PATH
```

Without any project, `pyfaro tools` still works: the activated environment and
`PATH` are searched, as in the listing above.

### Choose uv: `--uv`

`--uv <PATH>` names the uv executable (a path, or a name found on `PATH`).
It only affects uv; Ruff, ty and pytest are never resolved through it. A uv
that cannot be inspected is `unavailable` in the listing:

```console
$ pyfaro tools --uv /nonexistent/uv
uv: unavailable (could not find uv at /nonexistent/uv; fix --uv <path> or install uv (https://docs.astral.sh/uv/getting-started/installation/, or `curl -LsSf https://astral.sh/uv/install.sh | sh`))
ruff: ~/demo/.venv/bin/ruff (ruff 0.16.10)
ty: ~/demo/.venv/bin/ty (ty 0.0.85)
pytest: ~/demo/.venv/bin/pytest (pytest 9.1.1)
```

Pyfaro requires a stable uv `0.9.26` or newer.

## Precedence

For Ruff, ty and pytest, the first match wins:

1. the project environment: `.venv` in the project root, or the path in
   `UV_PROJECT_ENVIRONMENT` (absolute, or relative to the project root);
2. the environment named by `VIRTUAL_ENV`;
3. the `PATH`.

uv is the exception: it is resolved from `PATH` unless you pass `--uv`.

You can see the order at work. In the synced project, a stray `ruff` earlier on
`PATH` is ignored because the project environment comes first:

```console
$ PATH=/opt/stray/bin:$PATH pyfaro tools ruff
ruff: ~/demo/.venv/bin/ruff (ruff 0.16.10)
```

In a project without a `.venv`, the same stray binary is used:

```console
$ PATH=/opt/stray/bin:$PATH pyfaro tools ruff
ruff: /opt/stray/bin/ruff (ruff 0.0.1)
```

And `VIRTUAL_ENV` finds tools for a project that has no environment of its own:

```console
$ VIRTUAL_ENV=~/demo/.venv pyfaro tools ruff
ruff: ~/demo/.venv/bin/ruff (ruff 0.16.10)
```

!!! tip
    Because the project environment wins, `pyfaro lint`, `pyfaro type` and the
    other tool commands use exactly the binary shown here.

## What is checked

Each tool is probed with its own version command (`--version`, or `ty version`
for ty). Ruff, ty and pytest must print a parseable version; pytest is read
with the [PEP 440](https://packaging.python.org/en/latest/specifications/version-specifiers/)
scheme, so prereleases are fine. Pyfaro tests against Ruff 0.16.10, ty 0.0.85
and pytest 9.1.1 but does not reject other versions.

## Guarantees

- **No implicit sync.** Nothing is installed, no environment is created or
  updated, and no shell is started. Run [`pyfaro sync`](sync.md) yourself.
- **No writes.** The command only runs version probes.

## Errors and fixes

| Message starts with | Cause | Fix |
| --- | --- | --- |
| `error: <tool>: not found; install …` | The tool is not in `.venv`, `VIRTUAL_ENV` or `PATH` | `pyfaro add --dev <tool>`, then `pyfaro sync` ([add](add-remove.md), [sync](sync.md)) |
| `error: invalid value … for '[TOOL]'` | A name other than uv, ruff, ty, pytest | Use one of the four names |
| `uv: unavailable (…)` | `--uv` is wrong, or uv is missing or older than 0.9.26 | Install uv, or fix the `--uv` path |

## Exit codes

| Code | Meaning |
| --- | --- |
| `0` | Listing printed (even with `unavailable` entries), or the selected tool works |
| `1` | The selected tool is missing, failed its version probe or has an unrecognized version |
| `2` | Invalid arguments |

## See also

- [`pyfaro doctor`](doctor.md) — the full project report, which includes these
  tools as warnings.
- [`pyfaro sync`](sync.md) and [`pyfaro add`](add-remove.md) — install what is
  missing.
