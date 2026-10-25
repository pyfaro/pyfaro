# Running in the project environment

`pyfaro run [OPTIONS] -- <program> [args...]` delegates to `uv run` with an
explicit project directory. `pyfaro run [OPTIONS] <task> [-- extra...]` runs a
named task from `[tool.pyfaro.tasks]` the same way; see [Tasks](#tasks). The
program form is available in published `0.1.0-alpha.2` and later versions,
not in `0.1.0-alpha.1`; tasks are available in `0.4.0-alpha.1` and later.

```sh
pyfaro run -- python -m pytest
pyfaro run --locked -- python -c "print('ready')"
pyfaro run --project ./my-project -- python scripts/task.py --verbose
pyfaro run lint-all
```

The first `--` belongs to Pyfaro. Everything after it, including spaces,
Unicode, leading dashes, and another `--`, is passed as separate OS-native
arguments to uv and then the program. Pyfaro does not invoke a shell.
Without `--project`, Pyfaro finds the nearest `pyproject.toml` in the current
directory or its ancestors. `--project <PATH>` selects that exact directory
and requires its own `pyproject.toml`; it never falls back to a parent.
Discovery resolves symlinks first and does not change Pyfaro's process-wide
working directory. The program keeps the caller's working directory, so
relative script paths and arguments remain relative to the invocation point.
uv synchronizes the selected environment by default and
runs the program inside it. `--locked` refuses a missing or stale `uv.lock`;
`--frozen` uses an existing lock without checking it. These modes are
mutually exclusive.

An existing `.py` script passed by path, even outside the project, runs
through the project's Python interpreter. This includes scripts with PEP 723
inline metadata, but their inline dependencies are not installed; declare
needed packages in the project instead. A bare `.py` name matching a local
file chooses that file before `PATH`, as uv does. Otherwise uv resolves the
executable on `PATH`. Leading-dash script names work without being parsed as
Python options.

Use `--uv <executable>` to select uv and `--offline` to disable network
access. Pyfaro accepts stable uv versions >=0.9.26; CI tests 0.9.26. It
does not download uv or Python. Output streams directly, redirected stdin
is inherited, and an interactive Unix terminal is relayed to avoid
background-process-group suspension. A command accessing the terminal device
directly can be stopped by job control; Pyfaro terminates it with a diagnostic
rather than hanging. Run uv directly for commands that need stdin to be a TTY.
A numeric command failure is returned unchanged; cancellation stops the
delegated process tree and returns 130.

## Tasks

A task is a named command stored in `pyproject.toml`. It runs inside the same
project environment as `pyfaro run -- ...`, through `uv run`, with the same
streaming, stdin, exit-code and cancellation behavior.

```toml
[tool.pyfaro.tasks]
lint-all = ["ruff", "check", "."]
serve-docs = { command = ["python", "-m", "http.server", "8001"], cwd = "docs", env = { PYTHONUNBUFFERED = "1" }, description = "Serve the docs folder" }
changes = { shell = "git log --oneline | head -n 5" }
```

```sh
pyfaro run lint-all
pyfaro run lint-all -- --fix          # extra arguments are appended
pyfaro run serve-docs
pyfaro run changes
```

### Schema

- A task name matches `^[A-Za-z0-9][A-Za-z0-9_.-]*$` and has at most 64
  characters. Names containing a dot need quotes as TOML keys.
- A task is an array (shorthand for `command`) or a table with exactly one of
  `command` or `shell`, plus the optional `cwd`, `env` and `description`:
  - `command`: a non-empty array of non-empty strings, the program and its
    arguments. Pyfaro starts it directly, with no shell. Nothing is globbed
    and nothing is expanded: `*`, `~`, `$VAR` and `%VAR%` reach the program
    as written. An empty string cannot appear in the array; pass one as an
    extra argument instead.
  - `shell`: a non-empty string handed to a shell, see below. This is the only
    place where a shell is used, and only because you asked for it.
  - `cwd`: a path relative to the project root. The default is the project
    root, not the directory you ran `pyfaro` from. Absolute paths and paths
    that climb above the root with `..` are rejected, the directory must
    exist when the task runs, and it must still be inside the project after
    symlinks are resolved.
  - `env`: a table of strings added to the inherited environment, where task
    values win. Names are non-empty and have no `=` or NUL; values have no
    NUL. The variables are set on the uv process, so uv sees them as well as
    the program.
  - `description`: text shown when Pyfaro lists the tasks.
- Unknown task keys, a task with both or neither of `command` and `shell`, and
  values of the wrong type are errors reported before anything is launched,
  with the path of `pyproject.toml` and the dotted key, for example
  `tool.pyfaro.tasks.serve-docs.cwd`. Like every other Pyfaro setting, an
  invalid task table fails every command that reads the configuration.

### Tasks and programs never collide silently

Whether a command line means a task or a program depends only on the `--`:

- `pyfaro run <task> [-- extra...]` always means a task. An unknown name is an
  error (exit 1) that lists the defined tasks and says
  `to run a program, use pyfaro run -- <name>`. A name given outside any
  project is a usage error (exit 2) with the same hint.
- `pyfaro run -- <program> [args...]` always means a program and behaves as
  described above. If the program is spelled like a defined task, Pyfaro
  prints `pyfaro: running the program "<name>"; a task with the same name
  exists, run it with pyfaro run <name>` to stderr, then runs the program.
- `pyfaro run` with neither is a usage error (exit 2) that also lists the
  defined tasks.

Pyfaro never looks at `PATH` or the file system to decide. Adding, renaming or
removing a task cannot change what an existing `pyfaro run -- ...` does.

### Command tasks

`uv run --project <root> [--locked | --frozen] -- <argv...> <extra...>` is run
with the task's working directory and `env`. The arguments after the first
`--` on the Pyfaro command line are appended unchanged. Spaces, quotes, `$`,
`%`, `*`, `&`, `|`, `;`, Unicode, leading dashes and empty strings arrive as
separate literal arguments on Linux, macOS and Windows.
The `.py` convenience of the program form does not apply: write
`["python", "scripts/build.py"]`. Relative paths inside the task are relative
to its `cwd`.

### Shell tasks

`shell = "..."` is an explicit opt-in. Pyfaro runs the script through
`sh -c <script>` on Unix and `cmd.exe /d /s /c <script>` on Windows, still
inside the project environment, so pipes, `&&` and variable expansion work.

- Extra arguments after `--` are rejected (exit 1): appending them to a script
  would be injection by concatenation. Use a command task when a task takes
  arguments.
- On Windows a script that contains a double quote (`"`) is rejected before
  launch, because `cmd.exe` cannot receive one through argument quoting
  unchanged. Use a command task, or move the script into a file. Other
  tasks in the same project are unaffected. On Unix quotes work as in `sh`.
- The shell, and so the script, is platform-specific. A project that must run
  on both should prefer command tasks.

### Not included

Task dependencies, tasks that call other tasks, parallel execution, caching
and a `pyfaro tasks list` command are not part of this version. The design
and its rationale are recorded in
[ADR 0002](architecture/0002-task-invocation.md).
