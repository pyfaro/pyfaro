# Running in the project environment

`pyfaro run [OPTIONS] -- <program> [args...]` delegates to `uv run` with an
explicit project directory. It is in the unreleased `0.1.0-alpha.2`
development build, not the published `0.1.0-alpha.1` crate.

```sh
pyfaro run -- python -m pytest
pyfaro run --locked -- python -c "print('ready')"
pyfaro run --project ./my-project -- python scripts/task.py --verbose
```

The first `--` belongs to Pyfaro. Everything after it, including spaces,
Unicode, leading dashes, and another `--`, is passed as separate OS-native
arguments to uv and then the program. Pyfaro does not invoke a shell.
`--project <PATH>` defaults to the current directory. uv discovers that
project, which must contain `pyproject.toml`, synchronizes its environment by
default, and runs the program
inside it. `--locked` refuses a missing or stale `uv.lock`; `--frozen` uses
an existing lock without checking it. These modes are mutually exclusive.
Explicit `.py` script paths such as `./task.py` or `scripts/task.py` run through
the project's Python interpreter,
including scripts with PEP 723 inline metadata. This avoids uv switching them
to an isolated script environment. A bare `.py` command name retains normal
executable lookup on `PATH`, even if a same-named project file exists. Use
`./-task.py` for a leading-dash script name so Python does not parse it as an
option.

Use `--uv <executable>` to select uv and `--offline` to disable network
access. Pyfaro accepts stable uv versions >=0.9.26; CI tests 0.9.26. It
does not download uv or Python. Output streams directly, redirected stdin
is inherited, and an interactive Unix terminal is relayed to avoid
background-process-group suspension. Programs needing stdin to be an
actual TTY may need uv directly. A numeric command failure is returned
unchanged; cancellation stops the delegated process tree and returns 130.
