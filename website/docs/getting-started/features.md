# Features

Pyfaro wraps uv, Ruff, ty and pytest behind one command. Every command below
is implemented in `0.3.0-alpha.1` or on `main`.

## Project lifecycle

| Command | What it does | Delegates to |
| --- | --- | --- |
| [`create`](../commands/create.md) | Create a packaged library (`lib`), command-line app (`cli`) or FastAPI app (`fastapi`) | nothing, templates only |
| [`init`](../commands/init.md) | Preview or apply Pyfaro initialization of a new or existing project | uv |
| [`sync`](../commands/sync.md) | Create or update `uv.lock` and the project environment | uv |
| [`add` and `remove`](../commands/add-remove.md) | Edit dependencies and the lock file | uv |
| [`run`](../commands/run.md) | Run a command, or a named task from `[tool.pyfaro.tasks]`, in the project environment | uv |
| [`dev`](../commands/dev.md) | Run the development server (the `fastapi` preset runs Uvicorn with reload) or process, supervised | uv, Uvicorn |
| [`python`](../commands/python.md) | List, install or pin Python versions | uv |

## Code quality

| Command | What it does | Delegates to |
| --- | --- | --- |
| [`lint`](../commands/lint.md) | Lint with Ruff | Ruff |
| [`fmt`](../commands/fmt.md) | Format with Ruff, or check formatting | Ruff |
| [`type`](../commands/type.md) | Type-check | ty |
| [`check`](../commands/check.md) | Format check, lint and type check in one gate, with optional `--fix` | Ruff, ty |
| [`test`](../commands/test.md) | Run the tests | pytest |

## Diagnostics

| Command | What it does |
| --- | --- |
| [`tools`](../commands/tools.md) | Show where uv, Ruff, ty and pytest are found, and their versions |
| [`doctor`](../commands/doctor.md) | Read-only report on configuration, environment and tools |

## Design principles

- **No second resolver.** Pyfaro never resolves, installs or locks packages;
  uv does. See [How it works](../concepts/how-it-works.md).
- **Your configuration stays yours.** Ruff, ty and pytest read their own
  settings. Pyfaro only adds an optional `[tool.pyfaro]` table.
- **Nothing happens behind your back.** `check` writes nothing without
  `--fix`, `add` does not install, and Python is downloaded only by an
  explicit `python install`.
- **Honest exit codes.** Delegated commands return the tool's own code. See
  [Exit codes](../concepts/exit-codes.md).

!!! info "Planned, not available"
    `build` and workspaces are on the roadmap but are not implemented. They do
    not appear in `pyfaro --help`. Build with `uv build` for now.

## Next steps

- Start with the [first steps](first-steps.md).
- Build an API end to end: [A FastAPI app](../workflows/fastapi.md).
- Look up flags in the [command-line reference](../reference/cli.md).
- Learn the [concepts](../concepts/index.md) behind the commands.
