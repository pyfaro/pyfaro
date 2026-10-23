# Command line

Every Pyfaro command and its flags, as printed by `pyfaro --help` for the
development build, version `0.4.0-alpha.1`.

```console
$ pyfaro --help
Experimental Python project workflows powered by uv

Usage: pyfaro <COMMAND>

Commands:
  create  Create a packaged Python library, command-line or FastAPI project
  init    Preview or apply uv-backed project initialization
  sync    Synchronize the project environment through uv
  add     Add requirements to the project through uv
  remove  Remove requirements from the project through uv
  run     Run a named task or a command in the project environment through uv
  lint    Lint project files with Ruff
  fmt     Format project files with Ruff
  type    Type-check project files with ty
  test    Run pytest in the project environment
  check   Run format check, lint, and type check; optionally apply safe fixes first
  python  List, install, or pin Python versions through uv
  tools   Show resolved paths and versions of external tools
  doctor  Diagnose project configuration, environment, and tool availability
  help    Print this message or the help of the given subcommand(s)

Options:
  -h, --help     Print help
  -V, --version  Print version
```

Each section below lists the usage, arguments and options, and links to the
page that explains the command.

!!! info "Common options"
    - `--project <PROJECT>`: the project directory. See [Projects](../concepts/projects.md).
    - `--uv <UV>`: the uv executable, as a path or a name found on `PATH`
      (default `uv`).
    - `--offline`: forbid network access in uv.

    Not every command accepts every common option; the tables are exact.

## `pyfaro create`

Create a packaged Python library, command-line or FastAPI project. Tutorial:
[First steps](../getting-started/first-steps.md) · Guide: [`create`](../commands/create.md)

### `pyfaro create lib`

```text
Usage: pyfaro create lib [OPTIONS] <NAME>
```

| Argument or option | Description |
| --- | --- |
| `<NAME>` | Distribution name (also the default target directory name) |
| `--path <PATH>` | Target directory; its parent must already exist |
| `--python <PYTHON>` | Minimum Python version as `MAJOR.MINOR` (default `3.10`) |

### `pyfaro create cli`

```text
Usage: pyfaro create cli [OPTIONS] <NAME>
```

| Argument or option | Description |
| --- | --- |
| `<NAME>` | Distribution name (also the default target directory name) |
| `--path <PATH>` | Target directory; its parent must already exist |
| `--python <PYTHON>` | Minimum Python version as `MAJOR.MINOR` (default `3.10`) |
| `--package <PACKAGE>` | Import package name (defaults to the normalized distribution name) |
| `--command <COMMAND>` | Installed command name (defaults to the distribution name with dots replaced by hyphens) |

### `pyfaro create fastapi`

```text
Usage: pyfaro create fastapi [OPTIONS] <NAME>
```

| Argument or option | Description |
| --- | --- |
| `<NAME>` | Distribution name (also the default target directory name) |
| `--path <PATH>` | Target directory; its parent must already exist |
| `--python <PYTHON>` | Minimum Python version as `MAJOR.MINOR` (default `3.10`) |
| `--package <PACKAGE>` | Import package name (defaults to the normalized distribution name) |

## `pyfaro init`

Preview or apply uv-backed project initialization. Without `--apply`, nothing
is written. `--apply` is Linux-only; see [Platform support](../concepts/platform-support.md).
Guide: [`init`](../commands/init.md) · Workflow: [Adopt an existing project](../workflows/adopt-existing-project.md)

```text
Usage: pyfaro init [OPTIONS] [PATH]
```

| Argument or option | Description |
| --- | --- |
| `[PATH]` | Project directory (defaults to the current directory) |
| `--apply` | Apply the displayed plan; without this flag no files are written |
| `--uv <UV>` | uv executable (default `uv`) |
| `--offline` | Disable network access in uv |

## `pyfaro sync`

Synchronize the project environment through uv. Tutorial:
[First steps](../getting-started/first-steps.md) · Guide: [`sync`](../commands/sync.md)

```text
Usage: pyfaro sync [OPTIONS] [PATH]
```

| Argument or option | Description |
| --- | --- |
| `[PATH]` | Directory to start project discovery from (defaults to the current directory) |
| `--project <PROJECT>` | Explicit project directory; overrides discovery |
| `--locked` | Refuse to update an absent or stale `uv.lock` |
| `--frozen` | Use `uv.lock` without checking it against project metadata |
| `--uv <UV>` | uv executable (default `uv`) |
| `--offline` | Disable network access in uv |

`--locked` and `--frozen` are mutually exclusive.

## `pyfaro add`

Add requirements to the project through uv. The lock file is updated; the
environment is not synchronized. Guide: [`add` and `remove`](../commands/add-remove.md)

```text
Usage: pyfaro add [OPTIONS] <PACKAGES>...
```

| Argument or option | Description |
| --- | --- |
| `<PACKAGES>...` | Requirement specifiers |
| `--dev` | Use the development dependency group |
| `--group <GROUP>` | Use a named dependency group |
| `--project <PROJECT>` | Explicit project directory (otherwise discover from the current directory) |
| `--uv <UV>` | uv executable (default `uv`) |
| `--offline` | Disable network access in uv |

## `pyfaro remove`

Remove requirements from the project through uv. Guide: [`add` and `remove`](../commands/add-remove.md)

```text
Usage: pyfaro remove [OPTIONS] <PACKAGES>...
```

| Argument or option | Description |
| --- | --- |
| `<PACKAGES>...` | Dependency names |
| `--dev` | Use the development dependency group |
| `--group <GROUP>` | Use a named dependency group |
| `--project <PROJECT>` | Explicit project directory (otherwise discover from the current directory) |
| `--uv <UV>` | uv executable (default `uv`) |
| `--offline` | Disable network access in uv |

## `pyfaro run`

Run a named task or a command in the project environment through uv. Guide: [`run`](../commands/run.md)

```text
Usage: pyfaro run [OPTIONS] <TASK> [-- <ARGS>...]
       pyfaro run [OPTIONS] -- <PROGRAM> [ARGS]...
```

| Argument or option | Description |
| --- | --- |
| `<TASK>` | Task defined in `[tool.pyfaro.tasks]`; omit it to run a program after `--` |
| `<ARGS>...` | Program and arguments after `--`, or extra arguments for the task |
| `--project <PROJECT>` | Explicit project directory (otherwise discover from the current directory) |
| `--locked` | Refuse to update an absent or stale `uv.lock` |
| `--frozen` | Use `uv.lock` without checking it against project metadata |
| `--uv <UV>` | uv executable (default `uv`) |
| `--offline` | Disable network access in uv |

## `pyfaro python`

List, install or pin Python versions through uv. Guide: [`python`](../commands/python.md) ·
Workflow: [Python versions](../workflows/python-versions.md)

```text
Usage: pyfaro python [OPTIONS] <COMMAND>
```

The options below are accepted before the subcommand and also by each one.

| Option | Description |
| --- | --- |
| `--project <PROJECT>` | Exact project directory; otherwise use the current directory |
| `--uv <UV>` | uv executable (default `uv`) |
| `--offline` | Disable network access in uv |

### `pyfaro python list`

```text
Usage: pyfaro python list [OPTIONS] [REQUEST]
```

| Argument or option | Description |
| --- | --- |
| `[REQUEST]` | Python request to filter by |
| `--only-installed` | Show installed versions only |
| `--all-versions` | Include old patch versions |

### `pyfaro python install`

Explicitly download and install Python versions. This is the only command that
lets uv download an interpreter.

```text
Usage: pyfaro python install [OPTIONS] [TARGETS]...
```

| Argument or option | Description |
| --- | --- |
| `[TARGETS]...` | Python version requests; without targets uv uses `.python-version` |

### `pyfaro python pin`

```text
Usage: pyfaro python pin [OPTIONS] [REQUEST]
```

| Argument or option | Description |
| --- | --- |
| `[REQUEST]` | Python version request; omit to display the current pin |
| `--rm` | Remove the local pin |

## `pyfaro tools`

Show resolved paths and versions of external tools. Guide: [`tools`](../commands/tools.md) ·
Concept: [Environments and tools](../concepts/environments-and-tools.md)

```text
Usage: pyfaro tools [OPTIONS] [TOOL]
```

| Argument or option | Description |
| --- | --- |
| `[TOOL]` | Check one tool and fail if it is missing or incompatible. One of `uv`, `ruff`, `ty`, `pytest` |
| `--project <PROJECT>` | Explicit project directory; otherwise discover the nearest project |
| `--uv <UV>` | uv executable (a path or a name found on `PATH`) |

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

## `pyfaro lint`

Lint project files with Ruff. Guide: [`lint`](../commands/lint.md)

```text
Usage: pyfaro lint [OPTIONS] [PATHS]...
```

| Argument or option | Description |
| --- | --- |
| `[PATHS]...` | Files or directories relative to the current directory (default: the project) |
| `--project <PROJECT>` | Explicit project directory; otherwise discover the nearest project |

## `pyfaro fmt`

Format project files with Ruff. Guide: [`fmt`](../commands/fmt.md)

```text
Usage: pyfaro fmt [OPTIONS] [PATHS]...
```

| Argument or option | Description |
| --- | --- |
| `[PATHS]...` | Files or directories relative to the current directory (default: the project) |
| `--check` | Report files that would change without modifying them |
| `--project <PROJECT>` | Explicit project directory; otherwise discover the nearest project |

## `pyfaro type`

Type-check project files with ty. Guide: [`type`](../commands/type.md)

```text
Usage: pyfaro type [OPTIONS] [PATHS]...
```

| Argument or option | Description |
| --- | --- |
| `[PATHS]...` | Files or directories relative to the current directory (default: the project) |
| `--project <PROJECT>` | Explicit project directory; otherwise discover the nearest project |

## `pyfaro check`

Run format check, lint and type check; optionally apply safe fixes first.
Tutorial: [First steps](../getting-started/first-steps.md) · Guide: [`check`](../commands/check.md) ·
Workflow: [The quality gate](../workflows/quality-gate.md)

```text
Usage: pyfaro check [OPTIONS] [PATHS]...
```

| Argument or option | Description |
| --- | --- |
| `[PATHS]...` | Files or directories relative to the current directory (default: the project) |
| `--fix` | Apply safe Ruff fixes and formatting first, then verify without writing |
| `--project <PROJECT>` | Explicit project directory; otherwise discover the nearest project |

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

## Exit codes

See [Exit codes](../concepts/exit-codes.md).
