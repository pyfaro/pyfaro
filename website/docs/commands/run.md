# pyfaro run

`pyfaro run` runs a command inside the project's environment, by delegating to
`uv run` with an explicit project directory. Use it to run tools, scripts or
the project's own entry points without activating `.venv`.

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

Forgetting the separator is an error:

```console
$ pyfaro run python --version
error: unexpected argument 'python' found

Usage: pyfaro run [OPTIONS] -- <COMMAND>...
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
| `unexpected argument '…' found` | Put `--` before the command. |
| `no pyproject.toml found from …` | Run inside a project or pass `--project`. |
| `Failed to spawn: nosuchprog` | The program is not in the environment or on `PATH`. Add it with [`pyfaro add`](add-remove.md) and sync. |
| `the following required arguments were not provided: <COMMAND>...` | Give a command after `--`. |
| `--locked` failures | See [`pyfaro sync`](sync.md#2-locked-fail-if-the-lock-is-stale). |

## Exit codes

| Code | Meaning |
| --- | --- |
| `0` | The command succeeded. |
| `1` | A Pyfaro error such as no project found, or the command exited 1. |
| `2` | A usage error, or uv could not spawn the program. |
| other | The command's own status is forwarded unchanged. |
| `130` | Cancelled. The delegated process tree is stopped. |

## See also

- [`pyfaro sync`](sync.md): prepare the environment ahead of time.
- [`pyfaro test`](test.md): run pytest with Pyfaro's defaults.
- [`pyfaro add` and `remove`](add-remove.md): put the missing program in the project.
