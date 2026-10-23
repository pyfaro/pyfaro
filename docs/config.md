# Project configuration

Pyfaro reads `pyproject.toml` from the selected project root. No separate
Pyfaro metadata file is required. A project must have a standard `[project]`
table. The selected root is resolved to a physical path before configuration
is loaded, so nested invocations and symlinks use the same metadata. See the
[project selection rules](sync.md) for discovery and exact `--project` paths.

`[tool.pyfaro]` is optional for `sync` and `run`. Omitting it uses schema
version `1` without writing to the file. `init` explicitly adds this table:

```toml
[tool.pyfaro]
schema-version = 1
```

The supported settings are `schema-version`, which must be the integer `1`,
and the optional `[tool.pyfaro.tasks]` table. Unknown Pyfaro keys, other
schema versions, malformed TOML, and non-table `[project]`, `[tool]`, or
`[tool.pyfaro]` values fail before uv is launched. Errors identify the metadata path and invalid setting. Pyfaro
does not interpret or rewrite `[tool.ruff]`, `[tool.ty]`, `[tool.pytest]`,
`[tool.uv]`, or other tools' sections. `init` preserves comments and unrelated
TOML formatting when it adds its own settings.

The metadata path is resolved from the selected project root, not from the
location where a delegated command happens to run. Pyfaro adds no path-valued
settings in schema version `1`; uv and the individual tools retain ownership
of their own path semantics.

## Tasks

`[tool.pyfaro.tasks]` defines named commands for `pyfaro run <task>`:

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
| `cwd` | string | Directory relative to the project root; default is the root. Absolute paths and paths escaping the root are rejected. Must exist when the task runs. |
| `env` | table of strings | Variables added to the inherited environment; task values win. No interpolation. |
| `description` | string | Shown in task listings. |

Task validation happens with the rest of the configuration, so a mistake is
reported with the `pyproject.toml` path and the dotted key (for example
`tool.pyfaro.tasks.serve-docs.cwd`) before any tool is launched. The Windows
rejection of a `"` inside a shell script is the one check made when the task
is launched, because it depends on the platform. The semantics, including how
a task name differs from a program name, are in the
[run guide](run.md#tasks) and in
[ADR 0002](architecture/0002-task-invocation.md).
