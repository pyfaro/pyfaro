# Running the development process

`pyfaro dev` runs the project's development process, usually a web server,
inside the project environment and supervises it. It is configured in the
`[tool.pyfaro.dev]` table of `pyproject.toml`.

```toml
[tool.pyfaro.dev]
preset = "fastapi"              # optional; "fastapi" is the only preset
entry-point = "demo.main:app"   # required
args = ["--port", "8001"]       # optional array of strings
reload = true                   # optional boolean; needs a preset
```

```sh
pyfaro dev
pyfaro dev --project ./my-project --locked
pyfaro dev -- --port 9000 --log-level debug
pyfaro dev --verbose
pyfaro -v dev           # the same: --verbose is a global option
```

## Settings

| Key | Type | Meaning |
| --- | --- | --- |
| `preset` | string | `"fastapi"` runs the entry point with Uvicorn. Without a preset the entry point is a module run with `python -m`. |
| `entry-point` | string | Required. With `preset = "fastapi"`, an ASGI import string `package.module:attribute`. Without a preset, a module name `package.module` (no colon). |
| `args` | array of strings | Extra arguments appended after the entry point. May be empty. |
| `reload` | boolean | Whether Uvicorn reloads on file changes. Only valid with a preset; the `fastapi` preset defaults to `true`, and an explicit `false` wins. |

Pyfaro has no file watcher of its own. `reload` switches on Uvicorn's own
reloader; without a preset it is an error (`reload requires preset =
"fastapi"; Pyfaro does not watch files itself`). Unknown keys, a preset other
than `fastapi`, an entry point of the wrong shape, `args` that is not an array
of strings and a non-boolean `reload` are rejected with exit status 1 before
anything is launched. The message names the key, for example
`tool.pyfaro.dev.args must be an array of strings`. Every Pyfaro command
validates the table, so a mistake is reported even by `pyfaro sync`, and a
project without the table keeps working with every command except `dev`.

Without the table, `pyfaro dev` exits 1 and prints a minimal example together
with a link to this guide.

## What runs

Everything runs inside the project environment through
`uv run --project <root> [--locked | --frozen] -- ...`, with the project root
as the working directory. Precedence is: preset defaults, then
`[tool.pyfaro.dev]`, then the extra arguments after `--` on the command line.

With the `fastapi` preset:

```text
uv run --project <root> -- uvicorn <entry-point> --reload --reload-dir <root>/src <args...> <extra...>
```

`--reload` and `--reload-dir` are present while reload is on. The reload
directory is `<root>/src` when it exists, otherwise the project root. Because
extra arguments come last, `pyfaro dev -- --port 9000` overrides a configured
`--port`; Uvicorn uses the last value of an option.

Without a preset:

```text
uv run --project <root> -- python -m <entry-point> <args...> <extra...>
```

With the `fastapi` preset the project must declare `uvicorn`
(`[project].dependencies` or any `[dependency-groups]` list; extras, version
specifiers and markers are fine, and names are compared PEP 503 normalized).
Otherwise `pyfaro dev` exits 1 and suggests `pyfaro add "uvicorn[standard]"`.

## The FastAPI template

`pyfaro create fastapi acme-api` writes this table for you, with the entry
point following `--package` (`acme_api` by default):

```toml
[tool.pyfaro.dev]
preset = "fastapi"
entry-point = "acme_api.main:app"
```

So `pyfaro sync` followed by `pyfaro dev` serves the generated application at
http://127.0.0.1:8000/ with Uvicorn's reloader: editing `main.py` is picked up,
and a syntax error shows its traceback in the output until you fix it. Use
`pyfaro dev -- --port 8001` when the default port is taken.

## Options

| Option | Meaning |
| --- | --- |
| `--project <PATH>` | Use that exact project directory instead of discovery. |
| `--locked` / `--frozen` | Passed to `uv run`. Mutually exclusive. |
| `--uv <EXE>` | The uv executable (default `uv`). |
| `--offline` | Disable uv network access. |
| `-v`, `--verbose` | Also print the uv version and path and every command line, including the full `uv run` line with arguments that contain spaces quoted. This is Pyfaro's global option (see the [diagnostics guide](diagnostics.md)), so `pyfaro -v dev` works as well. |
| `-- <extra>...` | Arguments appended after the configured `args`. |

## Output

The child's standard output and error stream through unmodified: colors stay,
nothing is prefixed and nothing is buffered. Pyfaro's own lines go to standard
error and all start with `pyfaro dev: `:

```text
pyfaro dev: starting demo.main:app with uvicorn (reload on) at http://127.0.0.1:8000 - press Ctrl-C to stop
pyfaro dev: starting python -m demo.worker (press Ctrl-C to stop)
pyfaro dev: running /usr/bin/uv --version   (with --verbose)
pyfaro dev: using uv 0.9.26 at /usr/bin/uv   (with --verbose)
pyfaro dev: running /usr/bin/uv --no-python-downloads --no-progress run --project /work/demo -- uvicorn demo.main:app --reload --reload-dir /work/demo/src   (with --verbose)
```

The address in the first line is computed from the last `--host` and `--port`
(`--host=X` and `--port=N` work too), defaulting to `127.0.0.1` and `8000`.
With `--uds` or `--fd` the line names the socket or descriptor instead.

## Port check

With the `fastapi` preset Pyfaro tries to bind the effective host and port
before launching, unless `--uds` or `--fd` is present. If the port is taken it
exits 1 without starting anything:

```text
pyfaro dev: port 8000 on 127.0.0.1 is already in use
pyfaro dev: stop the process using it, run pyfaro dev -- --port 8001, or set args = ["--port", "8001"] in [tool.pyfaro.dev]
```

A `--port` value that is not an integer from 0 to 65535 is an error before
launch. Other bind failures are left for Uvicorn to report.

## Stopping and exit codes

| Situation | Output | Exit code |
| --- | --- | --- |
| The process exits 0 on its own | none | `0` |
| It exits with status `N` | `pyfaro dev: <runner> exited with status N` | `N` |
| It is killed by a signal | `pyfaro dev: <runner> was terminated (...)` | `1` |
| Ctrl-C | `pyfaro dev: stopped` | `130` |
| Configuration, missing Uvicorn, port | a message as above or `error: ...` | `1` |

`<runner>` is `uvicorn` with the preset and `python` otherwise. A status can
also be uv's own, for example when `--locked` finds a stale lock.

On Ctrl-C Pyfaro stops the whole process tree, including Uvicorn's reloader
and its worker, and leaves no process behind. On Unix the tree first receives
SIGINT and has about five seconds to shut down cleanly (other commands wait
only a fraction of a second) before it is killed. On an interactive Unix
terminal the terminal is relayed to the child as for `pyfaro run`.

### Ctrl-C on Windows

A Windows console delivers Ctrl-C to every process attached to it, Pyfaro
included, and Uvicorn's reloader uses that to restart its worker after each
file change (`os.kill(pid, signal.CTRL_C_EVENT)`). `pyfaro dev` therefore does
not take a Ctrl-C for a stop request at once:

- If the process tree ends by itself within about five seconds, that was a
  stop: `pyfaro dev: stopped`, exit `130`, and the job holding the tree is
  terminated to leave nothing behind.
- If the tree is still running after five seconds, or a new process started
  meanwhile (a restarted worker), the Ctrl-C was not meant to stop it. Pyfaro
  forgets it and keeps supervising, so editing a file under the reloader does
  not stop `pyfaro dev`.
- If part of the tree ended but nothing replaced it by the end of the window
  (Uvicorn's reloader can swallow a Ctrl-C after restarting a worker that had
  crashed, leaving the reloader alone), the Ctrl-C was a stop request that was
  only partly obeyed: the rest of the tree is terminated, as above.
- A second Ctrl-C while the first is still pending stops the tree, by
  terminating the job: use it for a process that ignores Ctrl-C. It does so at
  once when Pyfaro has watched the tree without a gap since the first Ctrl-C
  and nothing restarted. A restart is only noticed by sampling the job, and a
  worker is out of it for a few milliseconds, so on a busy machine a reload can
  pass between two samples and leave its Ctrl-C pending; the Ctrl-C of the next
  reload would then look like a second one. When Pyfaro could have missed a
  restart like that, a second Ctrl-C waits half a second (a second at most, if
  the machine stays busy) for the restart it may itself have caused, and only
  then terminates the tree. A third Ctrl-C does not wait.

While it supervises, `pyfaro dev` also opens the console (`CONOUT$`) every
20 ms, from a thread of its own (opening the console can block for tens of
milliseconds on a busy machine, which would blind the supervision). A console hands out a Ctrl-C that one of its processes generated, such
as Uvicorn's, only when something next uses the console; a terminal window
always does, but a console reached only through pipes (an IDE, a service, a
test harness) may hold the event back indefinitely, and Uvicorn's worker would
never restart. The open delivers it at once and has no other effect.

Other commands keep terminating their job immediately on Ctrl-C, as before.
