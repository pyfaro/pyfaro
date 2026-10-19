# Running in the project environment

`pyfaro run [OPTIONS] -- <program> [args...]` delegates to `uv run` with an
explicit project directory. It is available in published `0.1.0-alpha.2`
and later versions, not in `0.1.0-alpha.1`.

```sh
pyfaro run -- python -m pytest
pyfaro run --locked -- python -c "print('ready')"
pyfaro run --project ./my-project -- python scripts/task.py --verbose
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
