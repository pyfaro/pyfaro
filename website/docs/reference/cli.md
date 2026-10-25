---
description: "Every Pyfaro command and subcommand with its usage line, arguments, flags and defaults."
---

# Command line

Every Pyfaro command and subcommand, with its usage line, every argument and
flag, and its exit codes, as printed by `pyfaro --help` for version
`0.6.0-beta.1` (in development). A test compares this page with the program's own argument
definitions, so a command, flag or help text that changes without this page
failing the build is not possible. The configuration file is described in the
[configuration reference](configuration.md), the variables in
[Environment variables](environment-variables.md).

```console
$ pyfaro --help
Experimental Python project workflows powered by uv

Usage: pyfaro [OPTIONS] <COMMAND>

Commands:
  create       Create a packaged Python library, command-line or FastAPI project
  init         Preview or apply uv-backed project initialization
  sync         Synchronize the project environment through uv
  add          Add requirements to the project through uv
  remove       Remove requirements from the project through uv
  run          Run a named task or a command in the project environment through uv
  dev          Run the project's development server or process with supervision
  lint         Lint project files with Ruff
  fmt          Format project files with Ruff
  type         Type-check project files with ty
  test         Run pytest in the project environment
  check        Run format check, lint, and type check; optionally apply safe fixes first
  python       List, install, or pin Python versions through uv
  build        Build source and wheel distributions through uv
  tools        Show resolved paths and versions of external tools
  doctor       Diagnose project configuration, environment, and tool availability
  completions  Print the shell completion script for bash, zsh, fish or PowerShell
  help         Print this message or the help of the given subcommand(s)

Options:
  -v, --verbose  Print tool versions and every command line before it runs
  -h, --help     Print help
  -V, --version  Print version

Workflow:
  pyfaro create fastapi acme-api   # then: cd acme-api
  pyfaro sync                      # build the environment
  pyfaro run -- python --version   # run a command in the environment
  pyfaro check --fix               # format, lint and type-check
  pyfaro test                      # run pytest
  pyfaro dev                       # serve the app with auto-reload

Shell completion: pyfaro completions --help
Documentation: https://pyfaro.dev/
```

## Global options

These belong to `pyfaro` itself and work with every command.

| Option | Description |
| --- | --- |
| `-v`, `--verbose` | Print tool versions and every command line before it runs |
| `-h`, `--help` | Print help; `pyfaro help <command>` and `pyfaro <command> --help` print the help of one command |
| `-V`, `--version` | Print the version (`pyfaro --version`) |

`-v`/`--verbose` is accepted before the subcommand for every command, and
after it for every command except `test`. `pyfaro test` forwards everything
after `test` other than `--project` to pytest, so `pyfaro test -v` and
`pyfaro test --verbose` are pytest's options; use `pyfaro -v test` for
Pyfaro's trace. See [Verbose output](../workflows/troubleshooting.md#verbose-output).
With no arguments at all, Pyfaro prints the help to standard error and exits
`2`.

## How to read the tables

Each command section has the usage line, one row per argument and flag with
the help text of `pyfaro <command> --help` (a remark in brackets such as the
default may follow), and the exit codes. The meanings of the codes are the
contract in [Exit codes](../concepts/exit-codes.md): `0` success, `1` a
Pyfaro failure, `2` invalid usage, `130` cancellation, and any other
number is the status of the one tool that Pyfaro delegated to, forwarded
unchanged.

Common options keep one meaning everywhere:

- `--project <PROJECT>`: the project directory. See [Projects](../concepts/projects.md).
- `--uv <UV>`: the uv executable, as a path or a name found on `PATH`
  (default `uv`).
- `--offline`: forbid network access in uv.
- `--locked`, `--frozen`: strict lock handling on `sync`, `run` and `dev`.
  Mutually exclusive.

## `pyfaro create`

Create a packaged Python library, command-line or FastAPI project. Tutorial:
[First steps](../getting-started/first-steps.md) · Guide: [`create`](../commands/create.md)

```text
Usage: pyfaro create [OPTIONS] <COMMAND>
```

| Option | Description |
| --- | --- |
| `-v`, `--verbose` | Print tool versions and every command line before it runs |

### `pyfaro create lib`

```text
Usage: pyfaro create lib [OPTIONS] <NAME>
```

| Argument or option | Description |
| --- | --- |
| `<NAME>` | Distribution name (also the default target directory name) |
| `--path <PATH>` | Target directory; its parent must already exist |
| `--python <PYTHON>` | Minimum Python version as `MAJOR.MINOR` [default: `3.10`] |
| `-v`, `--verbose` | Print tool versions and every command line before it runs |

**Exit codes:** `0` the project was created; `1` any failure (no files are left behind); `2` usage error. `create` runs no tool, so no tool status is forwarded.

### `pyfaro create cli`

```text
Usage: pyfaro create cli [OPTIONS] <NAME>
```

| Argument or option | Description |
| --- | --- |
| `<NAME>` | Distribution name (also the default target directory name) |
| `--path <PATH>` | Target directory; its parent must already exist |
| `--python <PYTHON>` | Minimum Python version as `MAJOR.MINOR` [default: `3.10`] |
| `--package <PACKAGE>` | Import package name (defaults to the normalized distribution name) |
| `--command <COMMAND>` | Installed command name (defaults to the distribution name with dots replaced by hyphens) |
| `-v`, `--verbose` | Print tool versions and every command line before it runs |

**Exit codes:** `0` the project was created; `1` any failure (no files are left behind); `2` usage error. `create` runs no tool, so no tool status is forwarded.

### `pyfaro create fastapi`

```text
Usage: pyfaro create fastapi [OPTIONS] <NAME>
```

| Argument or option | Description |
| --- | --- |
| `<NAME>` | Distribution name (also the default target directory name) |
| `--path <PATH>` | Target directory; its parent must already exist |
| `--python <PYTHON>` | Minimum Python version as `MAJOR.MINOR` [default: `3.10`] |
| `--package <PACKAGE>` | Import package name (defaults to the normalized distribution name) |
| `-v`, `--verbose` | Print tool versions and every command line before it runs |

**Exit codes:** `0` the project was created; `1` any failure (no files are left behind); `2` usage error. `create` runs no tool, so no tool status is forwarded.

## `pyfaro init`

Preview or apply uv-backed project initialization. Without `--apply`, nothing
is written. `--apply` works on Linux, macOS and Windows.
Guide: [`init`](../commands/init.md) · Workflow: [Adopt an existing project](../workflows/adopt-existing-project.md)

```text
Usage: pyfaro init [OPTIONS] [PATH]
```

| Argument or option | Description |
| --- | --- |
| `[PATH]` | Project directory (defaults to the current directory) |
| `--apply` | Apply the displayed plan; without this flag no files are written |
| `--uv <UV>` | uv executable (a path or a name found on `PATH`) [default: `uv`] |
| `--offline` | Disable network access in uv |
| `-v`, `--verbose` | Print tool versions and every command line before it runs |

**Exit codes:** `0` the preview was shown or the plan was applied; with `--apply`, uv's own status, forwarded unchanged, when uv fails (`uv init` or `uv lock`); `1` a Pyfaro failure, including `--apply` outside Linux and a rollback that failed; `2` for a usage error and `130` when cancelled with Ctrl-C.

## `pyfaro sync`

Synchronize the project environment through uv. Tutorial:
[First steps](../getting-started/first-steps.md) · Guide: [`sync`](../commands/sync.md)

```text
Usage: pyfaro sync [OPTIONS] [PATH]
```

| Argument or option | Description |
| --- | --- |
| `[PATH]` | Directory to start project discovery from (defaults to the current directory) |
| `--project <PROJECT>` | Explicit project directory; otherwise discover the nearest project |
| `--locked` | Refuse to update an absent or stale `uv.lock` |
| `--frozen` | Use `uv.lock` without checking it against project metadata |
| `--uv <UV>` | uv executable (a path or a name found on `PATH`) [default: `uv`] |
| `--offline` | Disable network access in uv |
| `-v`, `--verbose` | Print tool versions and every command line before it runs |

`--locked` and `--frozen` are mutually exclusive.

**Exit codes:** `0` the environment is synchronized; uv's own status, forwarded unchanged, when uv fails; `1` a Pyfaro failure (no project, invalid configuration, uv missing or unsupported); `2` for a usage error and `130` when cancelled with Ctrl-C.

## `pyfaro add`

Add requirements to the project through uv. The lock file is updated; the
environment is not synchronized. Guide: [`add` and `remove`](../commands/add-remove.md)

```text
Usage: pyfaro add [OPTIONS] <PACKAGES>...
```

| Argument or option | Description |
| --- | --- |
| `<PACKAGES>...` | Requirement specifiers for add; dependency names for remove |
| `--dev` | Use the development dependency group |
| `--group <GROUP>` | Use a named dependency group |
| `--project <PROJECT>` | Explicit project directory; otherwise discover the nearest project |
| `--uv <UV>` | uv executable (a path or a name found on `PATH`) [default: `uv`] |
| `--offline` | Disable network access in uv |
| `-v`, `--verbose` | Print tool versions and every command line before it runs |

**Exit codes:** `0` the lock file was updated; uv's own status, forwarded unchanged, when uv fails; `1` a Pyfaro failure, including a failed rollback of `pyproject.toml` and `uv.lock`; `2` for a usage error and `130` when cancelled with Ctrl-C.

## `pyfaro remove`

Remove requirements from the project through uv. Guide: [`add` and `remove`](../commands/add-remove.md)

```text
Usage: pyfaro remove [OPTIONS] <PACKAGES>...
```

| Argument or option | Description |
| --- | --- |
| `<PACKAGES>...` | Requirement specifiers for add; dependency names for remove |
| `--dev` | Use the development dependency group |
| `--group <GROUP>` | Use a named dependency group |
| `--project <PROJECT>` | Explicit project directory; otherwise discover the nearest project |
| `--uv <UV>` | uv executable (a path or a name found on `PATH`) [default: `uv`] |
| `--offline` | Disable network access in uv |
| `-v`, `--verbose` | Print tool versions and every command line before it runs |

**Exit codes:** `0` the lock file was updated; uv's own status, forwarded unchanged, when uv fails; `1` a Pyfaro failure, including a failed rollback of `pyproject.toml` and `uv.lock`; `2` for a usage error and `130` when cancelled with Ctrl-C.

## `pyfaro run`

Run a named task or a command in the project environment through uv. Guide: [`run`](../commands/run.md)

```text
Usage: pyfaro run [OPTIONS] [TASK] [-- [ARGS]...]
```

| Argument or option | Description |
| --- | --- |
| `[TASK]` | Task defined in `[tool.pyfaro.tasks]`; omit it to run a program after `--` |
| `[ARGS]...` | Program and arguments after `--`, or extra arguments for the task |
| `--project <PROJECT>` | Explicit project directory; otherwise discover the nearest project |
| `--locked` | Refuse to update an absent or stale `uv.lock` |
| `--frozen` | Use `uv.lock` without checking it against project metadata |
| `--uv <UV>` | uv executable (a path or a name found on `PATH`) [default: `uv`] |
| `--offline` | Disable network access in uv |
| `-v`, `--verbose` | Print tool versions and every command line before it runs |

Either name a task (`pyfaro run <task> [-- <extra>...]`) or give a program after
`--` (`pyfaro run -- <program> [args...]`). Giving neither is a usage error.

**Exit codes:** the program's or task's own status, through `uv run` (uv reports a program killed by a signal as `128 + signal`); uv's status when uv itself fails; `1` a Pyfaro failure, including an invalid task; `2` neither a task nor `-- <program>`, or a name outside a project that looks like a forgotten `--`; `130` when cancelled with Ctrl-C.

## `pyfaro dev`

Run the project's development server or process with supervision. Guide:
[`dev`](../commands/dev.md)

```text
Usage: pyfaro dev [OPTIONS] [-- [EXTRA]...]
```

| Argument or option | Description |
| --- | --- |
| `[EXTRA]...` | Extra arguments after `--`, appended after the configured arguments |
| `--project <PROJECT>` | Explicit project directory; otherwise discover the nearest project |
| `--locked` | Refuse to update an absent or stale `uv.lock` |
| `--frozen` | Use `uv.lock` without checking it against project metadata |
| `--uv <UV>` | uv executable (a path or a name found on `PATH`) [default: `uv`] |
| `--offline` | Disable network access in uv |
| `-v`, `--verbose` | Print tool versions and every command line before it runs |

**Exit codes:** the process's own status, through `uv run` (`0` stays silent); `1` Pyfaro refuses to start (no `[tool.pyfaro.dev]` table, Uvicorn not declared, port in use); `2` usage error; `130` when stopped with Ctrl-C, after `pyfaro dev: stopped`.

## `pyfaro lint`

Lint project files with Ruff. Guide: [`lint`](../commands/lint.md)

```text
Usage: pyfaro lint [OPTIONS] [PATHS]...
```

| Argument or option | Description |
| --- | --- |
| `[PATHS]...` | Files or directories relative to the current directory (defaults to the project) |
| `--project <PROJECT>` | Explicit project directory; otherwise discover the nearest project |
| `-v`, `--verbose` | Print tool versions and every command line before it runs |

**Exit codes:** Ruff's own status, forwarded unchanged (`0` clean, non-zero for violations or errors); `1` also when Ruff is missing or unsupported; `2` for a usage error and `130` when cancelled with Ctrl-C.

## `pyfaro fmt`

Format project files with Ruff. Guide: [`fmt`](../commands/fmt.md)

```text
Usage: pyfaro fmt [OPTIONS] [PATHS]...
```

| Argument or option | Description |
| --- | --- |
| `[PATHS]...` | Files or directories relative to the current directory (defaults to the project) |
| `--check` | Report files that would change without modifying them |
| `--project <PROJECT>` | Explicit project directory; otherwise discover the nearest project |
| `-v`, `--verbose` | Print tool versions and every command line before it runs |

**Exit codes:** Ruff's own status, forwarded unchanged (with `--check`, non-zero when files would change); `1` also when Ruff is missing or unsupported; `2` for a usage error and `130` when cancelled with Ctrl-C.

## `pyfaro type`

Type-check project files with ty. Guide: [`type`](../commands/type.md)

```text
Usage: pyfaro type [OPTIONS] [PATHS]...
```

| Argument or option | Description |
| --- | --- |
| `[PATHS]...` | Files or directories relative to the current directory (defaults to the project) |
| `--project <PROJECT>` | Explicit project directory; otherwise discover the nearest project |
| `-v`, `--verbose` | Print tool versions and every command line before it runs |

**Exit codes:** ty's own status, forwarded unchanged (`0` clean, non-zero for diagnostics or errors); `1` also when ty is missing or unsupported; `2` for a usage error and `130` when cancelled with Ctrl-C.

## `pyfaro test`

Run pytest in the project environment. Tutorial:
[First steps](../getting-started/first-steps.md) · Guide: [`test`](../commands/test.md)

```text
Usage: pyfaro test [OPTIONS] [PYTEST_ARGS]...
```

| Argument or option | Description |
| --- | --- |
| `[PYTEST_ARGS]...` | Arguments forwarded verbatim to pytest (paths are relative to the project root) |
| `--project <PROJECT>` | Explicit project directory; otherwise discover the nearest project |

!!! note "`test` and `--verbose`"
    `pyfaro test` forwards everything after `test` other than `--project` to
    pytest, so `pyfaro test -v` and `pyfaro test --verbose` are pytest's
    options. Use `pyfaro -v test` for Pyfaro's trace.

**Exit codes:** pytest's own status, forwarded unchanged (`1` failures, `2` interrupted, `3` internal error, `4` usage error, `5` nothing collected); `1` also when pytest is missing or unsupported; Pyfaro's own usage errors are `2` too; `130` when cancelled with Ctrl-C.

## `pyfaro check`

Run format check, lint and type check; optionally apply safe fixes first.
Tutorial: [First steps](../getting-started/first-steps.md) · Guide: [`check`](../commands/check.md) ·
Workflow: [The quality gate](../workflows/quality-gate.md)

```text
Usage: pyfaro check [OPTIONS] [PATHS]...
```

| Argument or option | Description |
| --- | --- |
| `[PATHS]...` | Files or directories relative to the current directory (defaults to the project) |
| `--fix` | Apply safe Ruff fixes and formatting first, then verify without writing |
| `--project <PROJECT>` | Explicit project directory; otherwise discover the nearest project |
| `-v`, `--verbose` | Print tool versions and every command line before it runs |

**Exit codes:** `0` format check, lint and type check all passed; `1` any step failed or its tool is missing (all steps always run, and the tools' codes are not forwarded); `2` usage error; `130` when cancelled with Ctrl-C.

## `pyfaro python`

List, install or pin Python versions through uv. Guide: [`python`](../commands/python.md) ·
Workflow: [Python versions](../workflows/python-versions.md)

```text
Usage: pyfaro python [OPTIONS] <COMMAND>
```

The options below are accepted before the subcommand and also by each one.

| Option | Description |
| --- | --- |
| `--project <PROJECT>` | Exact project directory without discovery; otherwise uv discovers from the current directory |
| `--uv <UV>` | uv executable (a path or a name found on `PATH`) [default: `uv`] |
| `--offline` | Disable network access in uv |
| `-v`, `--verbose` | Print tool versions and every command line before it runs |

### `pyfaro python list`

```text
Usage: pyfaro python list [OPTIONS] [REQUEST]
```

| Argument or option | Description |
| --- | --- |
| `[REQUEST]` | Python version request to filter by |
| `--only-installed` | Show installed versions only |
| `--all-versions` | Include old patch versions |
| `--project <PROJECT>` | Exact project directory without discovery; otherwise uv discovers from the current directory |
| `--uv <UV>` | uv executable (a path or a name found on `PATH`) [default: `uv`] |
| `--offline` | Disable network access in uv |
| `-v`, `--verbose` | Print tool versions and every command line before it runs |

**Exit codes:** uv's own status, forwarded unchanged, when uv fails; `0` otherwise; `1` a Pyfaro failure; `2` for a usage error and `130` when cancelled with Ctrl-C.

### `pyfaro python install`

Explicitly download and install Python versions. This is the only command that
lets uv download an interpreter.

```text
Usage: pyfaro python install [OPTIONS] [TARGETS]...
```

| Argument or option | Description |
| --- | --- |
| `[TARGETS]...` | Python version requests; without targets uv uses `.python-version` |
| `--project <PROJECT>` | Exact project directory without discovery; otherwise uv discovers from the current directory |
| `--uv <UV>` | uv executable (a path or a name found on `PATH`) [default: `uv`] |
| `--offline` | Disable network access in uv |
| `-v`, `--verbose` | Print tool versions and every command line before it runs |

**Exit codes:** uv's own status, forwarded unchanged, when uv fails; `0` otherwise; `1` a Pyfaro failure; `2` for a usage error and `130` when cancelled with Ctrl-C.

### `pyfaro python pin`

```text
Usage: pyfaro python pin [OPTIONS] [REQUEST]
```

| Argument or option | Description |
| --- | --- |
| `[REQUEST]` | Python version request; omit to display the current pin |
| `--rm` | Remove the local pin |
| `--project <PROJECT>` | Exact project directory without discovery; otherwise uv discovers from the current directory |
| `--uv <UV>` | uv executable (a path or a name found on `PATH`) [default: `uv`] |
| `--offline` | Disable network access in uv |
| `-v`, `--verbose` | Print tool versions and every command line before it runs |

**Exit codes:** uv's own status, forwarded unchanged, when uv fails; `0` otherwise; `1` a Pyfaro failure; `2` for a usage error and `130` when cancelled with Ctrl-C.

## `pyfaro build`

Build source and wheel distributions through uv. Guide: [`build`](../commands/build.md)

```text
Usage: pyfaro build [OPTIONS]
```

| Option | Description |
| --- | --- |
| `--project <PROJECT>` | Explicit project directory; otherwise discover the nearest project |
| `--sdist` | Build only the source distribution |
| `--wheel` | Build only the wheel |
| `--out-dir <DIR>` | Directory for the artifacts, relative to the current directory (defaults to dist in the project) |
| `--clear` | Empty the output directory before building |
| `--uv <UV>` | uv executable (a path or a name found on `PATH`) [default: `uv`] |
| `--offline` | Disable network access in uv |
| `-v`, `--verbose` | Print tool versions and every command line before it runs |

**Exit codes:** `0` the artifacts were built; uv's own status, forwarded unchanged, when uv fails; `1` a Pyfaro failure; `2` usage error, including `--clear` with an output directory that is or contains the project; `130` when cancelled with Ctrl-C.

## `pyfaro tools`

Show resolved paths and versions of external tools. Guide: [`tools`](../commands/tools.md) ·
Concept: [Environments and tools](../concepts/environments-and-tools.md)

```text
Usage: pyfaro tools [OPTIONS] [TOOL]
```

| Argument or option | Description |
| --- | --- |
| `[TOOL]` | Check one tool and fail if it is missing or incompatible [possible values: `uv`, `ruff`, `ty`, `pytest`] |
| `--project <PROJECT>` | Explicit project directory; otherwise discover the nearest project |
| `--uv <UV>` | uv executable (a path or a name found on `PATH`) |
| `-v`, `--verbose` | Print tool versions and every command line before it runs |

**Exit codes:** `0` for the listing, even when a tool is `unavailable`, and for `tools <TOOL>` when the tool works; `1` for `tools <TOOL>` when the tool is missing or broken; `2` usage error (an unknown tool name); `130` when cancelled with Ctrl-C.

## `pyfaro doctor`

Diagnose project configuration, environment and tool availability. Tutorial:
[First steps](../getting-started/first-steps.md) · Guide: [`doctor`](../commands/doctor.md) ·
Workflow: [Troubleshooting](../workflows/troubleshooting.md)

```text
Usage: pyfaro doctor [OPTIONS]
```

| Option | Description |
| --- | --- |
| `--project <PROJECT>` | Explicit project directory; otherwise discover the nearest project |
| `--uv <UV>` | uv executable (a path or a name found on `PATH`) |
| `-v`, `--verbose` | Print tool versions and every command line before it runs |

**Exit codes:** `0` no `ERROR` was found (warnings are fine); `1` at least one `ERROR`; `2` usage error; `130` when cancelled with Ctrl-C. A failed tool version probe is a finding, so a tool's status is never forwarded.

## `pyfaro completions`

Print the shell completion script for bash, zsh, fish or PowerShell. Guide:
[`completions`](../commands/completions.md)

```text
Usage: pyfaro completions [OPTIONS] <SHELL>
```

| Argument or option | Description |
| --- | --- |
| `<SHELL>` | Shell to generate the completion script for [possible values: `bash`, `zsh`, `fish`, `powershell`] |
| `-v`, `--verbose` | Print tool versions and every command line before it runs |

**Exit codes:** `0` the script was printed; `2` a missing or unknown shell.

## Exit codes

See [Exit codes](../concepts/exit-codes.md) for the full contract. Each
command section above lists the codes that command can return.
