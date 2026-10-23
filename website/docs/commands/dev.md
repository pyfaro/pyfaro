# pyfaro dev

`pyfaro dev` runs the project's development process, usually a web server,
inside the project environment and supervises it: it streams the output
unchanged, tells you what it started, and on Ctrl+C stops the whole process
tree without leaving anything behind. What to run is configured in
[`[tool.pyfaro.dev]`](../concepts/configuration.md#toolpyfarodev).

## Quick example

```toml title="pyproject.toml"
[tool.pyfaro.dev]
preset = "fastapi"
entry-point = "demo.main:app"
```

```console
$ pyfaro dev
pyfaro dev: starting demo.main:app with uvicorn (reload on) at http://127.0.0.1:8000 - press Ctrl-C to stop
INFO:     Will watch for changes in these directories: ['/work/demo/src']
INFO:     Uvicorn running on http://127.0.0.1:8000 (Press CTRL+C to quit)
INFO:     Started reloader process [4121] using WatchFiles
INFO:     Application startup complete.
^C
INFO:     Shutting down
INFO:     Stopping reloader process [4121]
pyfaro dev: stopped
$ echo $?
130
```

The lines starting with `pyfaro dev: ` are Pyfaro's, written to standard
error. Everything else is Uvicorn's own output, passed through untouched.

## Scenarios

### 1. Configure the entry point

Add a `[tool.pyfaro.dev]` table to `pyproject.toml`. Without it, `pyfaro dev`
exits 1 and shows a minimal example:

```console
$ pyfaro dev
pyfaro dev: no [tool.pyfaro.dev] table in /work/demo/pyproject.toml
pyfaro dev: add one to run the project's development process, for example:

    [tool.pyfaro.dev]
    preset = "fastapi"
    entry-point = "my_package.main:app"

or, to run a module with python -m:

    [tool.pyfaro.dev]
    entry-point = "my_package.worker"

pyfaro dev: see https://pyfaro.dev/commands/dev/
```

| Key | Meaning |
| --- | --- |
| `preset` | Optional. `"fastapi"` runs the entry point with Uvicorn. |
| `entry-point` | Required. `package.module:attribute` with the preset; `package.module` (run as `python -m`) without. |
| `args` | Optional array of strings appended after the entry point. |
| `reload` | Optional boolean, only with a preset. The `fastapi` preset reloads by default; `reload = false` turns it off. |

### 2. The FastAPI preset

With `preset = "fastapi"`, Pyfaro runs Uvicorn through `uv run`:

```console
$ pyfaro dev --verbose
pyfaro dev: starting demo.main:app with uvicorn (reload on) at http://127.0.0.1:8000 - press Ctrl-C to stop
pyfaro dev: running /usr/bin/uv --no-python-downloads --no-progress run --project /work/demo -- uvicorn demo.main:app --reload --reload-dir /work/demo/src
```

Reloading is Uvicorn's own: Pyfaro adds no file watcher. It watches
`<project>/src` when that directory exists, otherwise the project root. The
project must declare `uvicorn`; otherwise `pyfaro dev` stops before launching:

```console
$ pyfaro dev
pyfaro dev: uvicorn is not declared in the project dependencies
pyfaro dev: add it with: pyfaro add "uvicorn[standard]"
```

### 3. Run a module

Without a preset the entry point is a module, run as `python -m` in the
project root:

```toml
[tool.pyfaro.dev]
entry-point = "demo.worker"
args = ["--queue", "default"]
```

```console
$ pyfaro dev
pyfaro dev: starting python -m demo.worker (press Ctrl-C to stop)
```

`reload` is rejected here, because Pyfaro does not watch files itself.

### 4. Extra arguments and precedence

Arguments after `--` are appended after the configured `args`, so they win:

```console
$ pyfaro dev -- --port 9000 --log-level debug
```

Preset defaults come first, then `[tool.pyfaro.dev]`, then the command line.

### 5. A port that is already in use

With the `fastapi` preset Pyfaro checks the effective host and port (the last
`--host` and `--port`, defaulting to `127.0.0.1` and `8000`) before it
launches anything:

```console
$ pyfaro dev
pyfaro dev: port 8000 on 127.0.0.1 is already in use
pyfaro dev: stop the process using it, run pyfaro dev -- --port 8001, or set args = ["--port", "8001"] in [tool.pyfaro.dev]
$ echo $?
1
```

A `--port` that is not a number from 0 to 65535 is also an error before
launch. The check is skipped with `--uds` or `--fd`.

### 6. When the process stops

| Situation | Exit code |
| --- | --- |
| The process exits 0 on its own | `0`, silently |
| It exits with status `N` | `N`, after `pyfaro dev: uvicorn exited with status N` (`python` without the preset) |
| You press Ctrl+C | `130`, after `pyfaro dev: stopped` |
| Invalid configuration, missing Uvicorn, busy port | `1` |

On Unix, Ctrl+C sends SIGINT to the whole tree (Uvicorn's reloader and its
worker included) and waits about five seconds for a clean shutdown before
killing what is left. On Windows the tree is terminated immediately. Either way
no process survives, and the port is free again.

## Options

| Option | Meaning |
| --- | --- |
| `--project <PATH>` | Use that exact directory instead of discovering the nearest project. |
| `--locked`, `--frozen` | Passed to `uv run`; mutually exclusive. |
| `--uv <EXE>` | Choose the uv executable. |
| `--offline` | Disable network access in uv. |
| `-v`, `--verbose` | Print the full uv command line before launching. |
| `-- <extra>...` | Arguments appended to the configured `args`. |

## Validation

The table is validated before anything starts, and the error names the key:

```console
$ pyfaro dev
error: invalid /work/demo/pyproject.toml: tool.pyfaro.dev.reload requires preset = "fastapi"; Pyfaro does not watch files itself
```

See [Configuration](../concepts/configuration.md#toolpyfarodev) for the full
list. The repository guide is [docs/dev.md](https://github.com/pyfaro/pyfaro/blob/main/docs/dev.md).
