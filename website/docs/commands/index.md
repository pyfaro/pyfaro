# Commands

Pyfaro wraps uv, Ruff, ty and pytest behind one command. Every command is a
thin, predictable layer: Pyfaro picks the project, builds the arguments, runs
the tool and forwards its exit status.

<span class="pf-badge">alpha</span> The commands below are implemented. Others
(such as `build` and workspaces) are planned and not available.

## All commands

| Command | What it does |
| --- | --- |
| [`create`](create.md) | Generate a packaged library, command-line or FastAPI project. |
| [`init`](init.md) | Preview or apply uv-backed initialization of an existing or bare project (apply is Linux-only). |
| [`sync`](sync.md) | Synchronize the project environment through uv. |
| [`add`, `remove`](add-remove.md) | Add or remove dependencies through uv, without syncing. |
| [`run`](run.md) | Run a command, or a named task, in the project environment. |
| [`dev`](dev.md) | Run the development server or process with supervision. |
| [`python`](python.md) | List, install or pin Python versions through uv. |
| [`tools`](tools.md) | Show resolved paths and versions of the external tools. |
| [`doctor`](doctor.md) | Diagnose project configuration, environment and tool availability. |
| [`completions`](completions.md) | Print the tab-completion script for Bash, Zsh, Fish or PowerShell. |
| [`lint`](lint.md) | Lint project files with Ruff. |
| [`fmt`](fmt.md) | Format project files with Ruff. |
| [`type`](type.md) | Type-check project files with ty. |
| [`check`](check.md) | Run format check, lint and type check, optionally applying safe fixes first. |
| [`test`](test.md) | Run pytest in the project environment. |

## Project setup

<div class="grid cards" markdown>

- :material-folder-plus: **[create](create.md)**: start a new library, CLI or FastAPI app.
- :material-clipboard-check: **[init](init.md)**: adopt an existing project.
- :material-sync: **[sync](sync.md)**: build the environment.
- :material-package-variant: **[add and remove](add-remove.md)**: manage dependencies.
- :material-play: **[run](run.md)**: run anything, or a named task, in the environment.
- :material-server: **[dev](dev.md)**: run the development server.
- :material-language-python: **[python](python.md)**: manage interpreters.

</div>

## How project selection works

Most commands find the nearest `pyproject.toml` in the current directory or
its ancestors. Pass `--project <PATH>` to select an exact directory instead.
`init` is the exception: it takes the directory to initialize as a positional
path and never searches upward, and `create` writes a new directory.

## Common flags

| Flag | Meaning |
| --- | --- |
| `--uv <path>` | Choose the uv executable (default `uv` from `PATH`). |
| `--offline` | Disable uv network access. |
| `--locked`, `--frozen` | Strict lock handling on `sync`, `run` and `dev`. Mutually exclusive. |

uv 0.9.26 or newer is required at runtime.
