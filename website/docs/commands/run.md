---
description: "Run a command or a named task inside the project environment with pyfaro run, with no implicit shell."
---

# pyfaro run

`pyfaro run` runs a command inside the project's environment, by delegating to
`uv run` with an explicit project directory. Use it to run tools, scripts or
the project's own entry points without activating `.venv`. It also runs
[named tasks](#named-tasks) that you define in `pyproject.toml`.

| Form | Meaning |
| --- | --- |
| `pyfaro run <task> [-- extra...]` | Run the task `<task>` from `[tool.pyfaro.tasks]`. |
| `pyfaro run -- <program> [args...]` | Run a program. Everything after `--` is the command. |

## Quick example

```console
$ pyfaro run -- python --version
Python 3.13.11
$ pyfaro run -- python -m pytest -q
.                                                                        [100%]
1 passed in 0.01s
```

## Scenarios

### 1. The `--` separator

The first `--` belongs to Pyfaro. Everything after it is the command, passed
as separate arguments with no shell involved, so spaces, Unicode, leading
dashes and even another `--` arrive untouched:

```console
$ pyfaro run -- python -c "import acme_widget; print(acme_widget.hello())"
Hello from acme-widget!
$ pyfaro run -- ruff --version
ruff 0.16.10
```

Forgetting the separator is an error. A name before the `--` is always a
[task](#named-tasks), so Pyfaro tells you how to run a program instead:

```console
$ pyfaro run python
error: no task named "python" in ~/work/acme-widget/pyproject.toml
no tasks are defined under [tool.pyfaro.tasks]
to run a program, use pyfaro run -- python
```

### 2. Run a `.py` script

An existing `.py` path runs through the project's interpreter. Arguments after
it go to the script:

```console
$ cat hello.py
import sys, os
print("args:", sys.argv[1:])
print("cwd:", os.getcwd())
$ pyfaro run -- hello.py --verbose -x
args: ['--verbose', '-x']
cwd: ~/work/acme-widget
$ pyfaro run -- ../../../hello.py a -- b
args: ['a', '--', 'b']
```

A bare `.py` name that matches a local file wins over a program on `PATH`. The
script may live outside the project and still run with the project's Python:

```console
$ pyfaro run --project ../acme-widget -- o.py
outside
```

!!! note
    PEP 723 inline metadata in a script is not honored. Its inline
    dependencies are not installed, so declare them in the project.

### 3. Where does the command run? (cwd semantics)

The command keeps **your** working directory. Pyfaro selects the project but
does not `cd` into it, so relative paths mean what they mean at the prompt:

```console
$ cd other        # no pyproject.toml here
$ pyfaro run --project ../acme-widget -- python -c "import os; print(os.getcwd())"
~/work/other
```

### 4. `--locked` and `--frozen`

By default uv synchronizes the environment before running (and may update
`uv.lock`). Make it strict:

- `--locked` refuses a missing or stale `uv.lock`.
- `--frozen` uses an existing lock without checking it.

They are mutually exclusive.

```console
$ pyfaro run --frozen --offline -- python -c "print('frozen')"
frozen
$ pyfaro run --locked --frozen -- python -V
error: the argument '--locked' cannot be used with '--frozen'
```

`--offline` disables network access and `--uv <path>` picks the uv binary.
uv 0.9.26 or newer is required. Pyfaro never downloads Python.

### 5. Exit codes pass through

```console
$ pyfaro run -- python -c "import sys; sys.exit(7)"
error: uv run failed (exit status: 7); see uv's output above
$ echo $?
7
```

## Named tasks

A task gives a name to a command you run often. Define it under
`[tool.pyfaro.tasks]` in `pyproject.toml`:

```toml
[tool.pyfaro.tasks]
lint-all = ["ruff", "check", "."]
serve-docs = { command = ["python", "-m", "http.server", "8001"], cwd = "docs", env = { PYTHONUNBUFFERED = "1" }, description = "Serve the docs folder" }
changes = { shell = "git log --oneline | head -n 5" }
```

```console
$ pyfaro run lint-all
All checks passed!
$ pyfaro run lint-all -- --fix
All checks passed!
```

A task runs exactly like `pyfaro run -- ...` does: in the project environment,
through `uv run`, with output streamed, stdin passed through, the command's
exit code returned unchanged and Ctrl-C stopping the whole process tree.

### 6. Short form and table form

An array is the short form: the program followed by its arguments. A table
adds options, and has exactly one of `command` (the same array) or `shell`:

| Key | Meaning |
| --- | --- |
| `command` | Program and arguments, a non-empty array of non-empty strings. |
| `shell` | A script for a shell. See [shell tasks](#8-shell-tasks-are-an-explicit-opt-in). |
| `cwd` | Directory relative to the project root. Default: the project root. |
| `env` | Variables added to the environment; task values win. |
| `description` | Shown when Pyfaro lists the tasks. |

Task names use letters, digits, `_`, `.` and `-`, start with a letter or a
digit and have at most 64 characters. Mistakes are reported before anything
runs, with the file and the key:

```console
$ pyfaro run serve-docs
error: invalid ~/work/acme-widget/pyproject.toml: unknown task setting: tool.pyfaro.tasks.serve-docs.colour (supported: command, shell, cwd, env, description)
```

### 7. Arguments are literal

A command task never goes through a shell. Pyfaro hands every element to uv as
a separate argument, so nothing is globbed or expanded: `*`, `~`, `$HOME`
and `%PATH%` reach the program as written, on Linux, macOS and Windows alike.
Arguments after the first `--` on the command line are appended:

```console
$ pyfaro run lint-all -- --select I src
```

Spaces, quotes, `&`, `|`, `;`, Unicode, leading dashes and empty strings
arrive untouched.

By default a task runs in the **project root**, even if you start `pyfaro`
from a subdirectory. `cwd` moves it to another directory, relative to the
project root. Absolute paths and paths that climb above the root are
rejected, and the directory must exist when the task runs. `env` is added to
the inherited environment (the task's value wins) with no interpolation.

### 8. Shell tasks are an explicit opt-in

Use `shell` when you really want pipes, `&&` or variable expansion. Pyfaro
runs the script with `sh -c` on Unix and `cmd.exe /d /s /c` on Windows, inside
the project environment:

```console
$ pyfaro run changes
a1b2c3d Add the greeting
9f8e7d6 Initial commit
```

Because the script is handed to a shell, Pyfaro refuses to append anything
to it:

```console
$ pyfaro run changes -- --stat
error: task "changes" is a shell task and does not accept extra arguments; remove them, or define a command task to pass arguments
```

On Windows a script containing a double quote (`"`) is rejected before
launch, because `cmd.exe` cannot receive one through argument quoting
unchanged. Use a command task or a script file instead. Scripts are shell
specific: a project that must work on every platform should prefer command
tasks.

### 9. Tasks and programs never collide silently

The `--` decides. A name before it is always a task; a program always comes
after it:

```console
$ pyfaro run test              # the task "test"
$ pyfaro run -- test           # the program "test"
pyfaro: running the program "test"; a task with the same name exists, run it with pyfaro run test
```

An unknown task name is an error that lists the defined tasks and the hint
`to run a program, use pyfaro run -- <name>`. With neither a task nor `--`,
Pyfaro exits with status 2 and lists the tasks:

```console
$ pyfaro run
error: the following required arguments were not provided:
  <TASK> or -- <PROGRAM>...

Usage: pyfaro run [OPTIONS] <TASK> [-- <ARGS>...]
       pyfaro run [OPTIONS] -- <PROGRAM> [ARGS]...

Tasks defined in ~/work/acme-widget/pyproject.toml:
  lint-all    ruff check .
  serve-docs  Serve the docs folder
  changes     shell: git log --oneline | head -n 5

For more information, try '--help'.
```

Pyfaro never consults `PATH` or the file system to decide, so adding or
renaming a task cannot change what an existing `pyfaro run -- ...` does.
Task dependencies, tasks that call tasks, parallel runs and a task listing
command are not available yet. The design is recorded in
[ADR 0002](https://github.com/pyfaro/pyfaro/blob/main/docs/architecture/0002-task-invocation.md).

## Project selection

| Form | Meaning |
| --- | --- |
| `pyfaro run -- …` | Nearest `pyproject.toml` in the current directory or its ancestors. |
| `pyfaro run --project <PATH> -- …` | That exact directory, which must hold a `pyproject.toml`. No fallback to a parent. |

There is no positional path, because everything after `--` is the command.
Symlinks are resolved before discovery:

```console
$ pyfaro run -- python -V
error: no pyproject.toml found from .; use --project <PATH>
```

## What it writes, and what it never writes

**Writes** (through uv, unless you pass `--locked` or `--frozen`): the
project's `.venv` and `uv.lock`, because `uv run` synchronizes first. The
command you run may of course write whatever it wants.

**Never writes:** `pyproject.toml`, a second lockfile or a downloaded Python.
Output is streamed live and redirected stdin is passed through.

!!! tip
    On an interactive terminal Pyfaro relays the terminal so the command is
    not suspended as a background process. A program that opens the terminal
    device directly can still be stopped by job control. Pyfaro ends it with
    a diagnostic instead of hanging. For commands that need a real TTY on
    stdin, run `uv run` directly.

## Errors you may hit

| Message | Fix |
| --- | --- |
| `no task named "…" in …` | The name before `--` is a task. Define it, or run a program with `pyfaro run -- <name>`. |
| `unexpected argument '…' found` | Put `--` before the arguments, for example `pyfaro run <task> -- --flag`. |
| `… is a shell task and does not accept extra arguments` | Remove the arguments after `--`, or use a command task. |
| `invalid …/pyproject.toml: tool.pyfaro.tasks…` | Fix the key it names; see [named tasks](#named-tasks). |
| `no pyproject.toml found from …` | Run inside a project or pass `--project`. |
| `Failed to spawn: nosuchprog` | The program is not in the environment or on `PATH`. Add it with [`pyfaro add`](add-remove.md) and sync. |
| `the following required arguments were not provided: <TASK> or -- <PROGRAM>...` | Name a task, or give a program after `--`. |
| `--locked` failures | See [`pyfaro sync`](sync.md#2-locked-fail-if-the-lock-is-stale). |

## Exit codes

| Code | Meaning |
| --- | --- |
| `0` | The command succeeded. |
| `1` | A Pyfaro error such as no project found, an unknown task or an invalid task, or the command exited 1. |
| `2` | A usage error, or uv could not spawn the program. |
| other | The command's own status is forwarded unchanged. |
| `130` | Cancelled. The delegated process tree is stopped. |

## See also

- [`pyfaro sync`](sync.md): prepare the environment ahead of time.
- [`pyfaro test`](test.md): run pytest with Pyfaro's defaults.
- [`pyfaro add` and `remove`](add-remove.md): put the missing program in the project.
