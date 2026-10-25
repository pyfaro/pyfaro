# Configuration

Pyfaro reads `pyproject.toml` from the selected [project](projects.md) root.
There is no separate Pyfaro file.

!!! tip "Looking for the exact rules?"
    This page explains the ideas. The [configuration reference](../reference/configuration.md)
    lists every key with its type, default, validation rule and error message,
    with copyable examples that the test suite checks.

## `[tool.pyfaro]`

The table is optional. Without it, Pyfaro behaves as if it contained the
default values, and never writes to your file.

```toml
[tool.pyfaro]
schema-version = 1
```

`pyfaro create` writes this table, and `pyfaro init --apply` adds it to an
existing project.

| Key | Type | Meaning |
| --- | --- | --- |
| `schema-version` | integer | Version of the configuration format. Must be `1`. |

`schema-version` exists so that future versions can change the format without
breaking existing projects. The other tables under `[tool.pyfaro]` are
[`tasks`](#toolpyfarotasks) and [`dev`](#toolpyfarodev).

## `[tool.pyfaro.tasks]`

Named commands for [`pyfaro run <task>`](../commands/run.md#named-tasks). The
table is optional.

```toml
[tool.pyfaro.tasks]
lint-all = ["ruff", "check", "."]
serve-docs = { command = ["python", "-m", "http.server", "8001"], cwd = "docs", env = { PYTHONUNBUFFERED = "1" }, description = "Serve the docs folder" }
changes = { shell = "git log --oneline | head -n 5" }
```

| Key | Type | Meaning |
| --- | --- | --- |
| `<name>` | array of strings, or table | A task. Names match `^[A-Za-z0-9][A-Za-z0-9_.-]*$` and have at most 64 characters. An array is shorthand for `command`. |
| `command` | non-empty array of non-empty strings | The program and its arguments, started without a shell. Exactly one of `command` and `shell`. |
| `shell` | non-empty string | A script for `sh -c` (Unix) or `cmd.exe /d /s /c` (Windows). Explicit opt-in. |
| `cwd` | string | Directory relative to the project root. Default: the project root. Absolute paths and paths escaping the root are rejected; it must exist when the task runs. |
| `env` | table of strings | Variables added to the inherited environment. Task values win. No interpolation. |
| `description` | string | Shown when Pyfaro lists the tasks. |

Unknown task keys, a task with both or neither of `command` and `shell`, and
wrong types are errors that name the dotted key, for example
`tool.pyfaro.tasks.serve-docs.cwd`.

## `[tool.pyfaro.dev]`

`pyfaro dev` reads what to run from this table. It is optional; every other
command works without it.

```toml
[tool.pyfaro.dev]
preset = "fastapi"              # optional; the only preset
entry-point = "demo.main:app"   # required
args = ["--port", "8001"]       # optional array of strings
reload = true                   # optional boolean; needs a preset
```

| Key | Type | Meaning |
| --- | --- | --- |
| `preset` | string | `"fastapi"` runs the entry point with Uvicorn. |
| `entry-point` | string | Required. `package.module:attribute` with the preset, a module name `package.module` (run as `python -m`) without. |
| `args` | array of strings | Appended after the entry point; command-line extras come after them. |
| `reload` | boolean | Uvicorn's reload. Only with a preset; defaults to `true` for `fastapi`, and `false` wins. |

Pyfaro has no file watcher, so `reload` without a preset is an error. See
[`pyfaro dev`](../commands/dev.md) for the commands that run.

## Validation

Pyfaro validates the file before it launches any tool. These cases fail with
exit code 1 and an error that names the file and the setting:

- malformed TOML;
- a missing `[project]` table, or one that is not a table;
- `[tool]` or `[tool.pyfaro]` that is not a table;
- an unknown key under `[tool.pyfaro]` or `[tool.pyfaro.dev]`;
- an invalid `[tool.pyfaro.dev]` value: a preset other than `fastapi`, a
  missing or malformed `entry-point`, `args` that is not an array of strings,
  a non-boolean `reload`, or `reload` without a preset;
- a `schema-version` other than `1`;
- an invalid task under `[tool.pyfaro.tasks]`.

`pyfaro doctor` runs the same validation and reports the result.

## Other tools' settings

Pyfaro does not read, interpret or rewrite `[tool.ruff]`, `[tool.ty]`,
`[tool.pytest]`, `[tool.uv]` or any other section. Each tool owns its own
configuration, so existing settings keep working. When `pyfaro init --apply`
adds its table, it preserves comments and unrelated formatting.

!!! note "Conflicting configuration files"
    `doctor` warns when a tool has settings in more than one place, for
    example `pytest.ini` next to `[tool.pytest.ini_options]`, since the tool
    will only use one of them.

## Next steps

- [Configuration reference](../reference/configuration.md): every key, default and error.
- [`pyfaro init`](../commands/init.md) to adopt an existing project.
- [Adopt an existing project](../workflows/adopt-existing-project.md).
- [Projects](projects.md).
