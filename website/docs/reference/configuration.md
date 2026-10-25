# Configuration reference

Every setting Pyfaro reads, with its type, default, rule and the error you get
when the rule is broken. Pyfaro reads `pyproject.toml` from the selected
[project](../concepts/projects.md) root and nothing else; the settings live
under `[tool.pyfaro]`. For the ideas behind them, see
[Configuration](../concepts/configuration.md).

Every TOML example on this page is checked by the test suite against Pyfaro's
own validation, so you can copy it. The examples that are meant to be rejected
start with `# invalid:` and the start of the error message.

## All settings at a glance

```toml
[project]
name = "demo"
version = "0.1.0"
requires-python = ">=3.10"
dependencies = ["fastapi", "uvicorn[standard]"]

[tool.pyfaro]
schema-version = 1

[tool.pyfaro.dev]
preset = "fastapi"
entry-point = "demo.main:app"
args = ["--port", "8001"]
reload = true

[tool.pyfaro.tasks]
lint-all = ["ruff", "check", "."]
serve-docs = { command = ["python", "-m", "http.server", "8001"], cwd = "docs", env = { PYTHONUNBUFFERED = "1" }, description = "Serve the docs folder" }
changes = { shell = "git log --oneline | head -n 5" }
```

The whole `[tool.pyfaro]` table is optional. Without it Pyfaro behaves as if it
contained `schema-version = 1`, and it never writes to your file.
`pyfaro create` writes the table and `pyfaro init --apply` adds it.

## How errors look

Pyfaro validates the whole file before it starts any tool. A rejected file
exits with code `1` and prints one line on standard error that names the file
and the setting:

```console
$ pyfaro sync
error: invalid /home/you/demo/pyproject.toml: tool.pyfaro.schema-version must be the integer 1
```

The message column in the tables below is the part after `invalid <path>: `.
Messages for a value that has the wrong shape name the dotted key, so you can
find it. Every command that reads the project validates the file, so a mistake
is reported even by `pyfaro sync`; `pyfaro doctor` shows the same finding in
its report.

| Problem | Message |
| --- | --- |
| Not valid TOML | `invalid <path>: TOML parse error at line …` |
| No `[project]` table | `expected a standard [project] table in pyproject.toml` |
| `[tool]` is not a table | `tool must be a TOML table` |
| `[tool.pyfaro]` is not a table | `[tool.pyfaro] must be a table` |
| Unknown key in `[tool.pyfaro]` | `unknown [tool.pyfaro] setting: <key>` |

## `[tool.pyfaro]`

| Key | Type | Default | Rule | Error |
| --- | --- | --- | --- | --- |
| `schema-version` | integer | `1` | Must be the integer `1`. It exists so a future format can change without breaking projects. | `tool.pyfaro.schema-version must be the integer 1` |
| `dev` | table | none | See [`[tool.pyfaro.dev]`](#toolpyfarodev). | |
| `tasks` | table | none | See [`[tool.pyfaro.tasks]`](#toolpyfarotasks). | |

Any other key is rejected:

```toml
# invalid: unknown [tool.pyfaro] setting: tasklist
[tool.pyfaro]
tasklist = 1
```

```toml
# invalid: tool.pyfaro.schema-version must be the integer 1
[tool.pyfaro]
schema-version = 2
```

## `[tool.pyfaro.dev]`

Read by [`pyfaro dev`](../commands/dev.md) only; the table is validated by
every command. It must be a table (`tool.pyfaro.dev must be a table`), and an
unknown key is rejected with `unknown [tool.pyfaro.dev] setting: <key>`.

| Key | Type | Default | Rule | Error |
| --- | --- | --- | --- | --- |
| `preset` | string | none | The only preset is `"fastapi"`, which runs the entry point with Uvicorn. | `tool.pyfaro.dev.preset must be the string "fastapi"` |
| `entry-point` | string | none, **required** | With `preset = "fastapi"`, an ASGI import string `package.module:attribute`. Without a preset, a module name `package.module`, run as `python -m`. Each part is a Python identifier. | `tool.pyfaro.dev.entry-point is required`, `… must be a string`, `… must be an ASGI import string like "package.module:app" with the fastapi preset, found "…"`, `… must be a module name like "package.module" (run as python -m) without a preset, found "…"` |
| `args` | array of strings | `[]` | Appended after the entry point; the arguments after `--` on the command line come last. | `tool.pyfaro.dev.args must be an array of strings` |
| `reload` | boolean | `true` with the `fastapi` preset | Switches on Uvicorn's reloader. Only valid with a preset, because Pyfaro has no file watcher. An explicit `false` wins over the preset's default. | `tool.pyfaro.dev.reload must be a boolean`, `tool.pyfaro.dev.reload requires preset = "fastapi"; Pyfaro does not watch files itself` |

With the `fastapi` preset the project must also declare `uvicorn`; that check
happens when `pyfaro dev` starts. The commands that run, and the precedence
between the preset, these settings and the command line, are in the
[`dev` command guide](../commands/dev.md).

A web application:

```toml
[tool.pyfaro.dev]
preset = "fastapi"
entry-point = "demo.main:app"
args = ["--port", "8001"]
reload = false
```

A background worker, which is a module run with `python -m`:

```toml
[tool.pyfaro.dev]
entry-point = "demo.worker"
args = ["--queue", "default"]
```

Mistakes:

```toml
# invalid: tool.pyfaro.dev.entry-point is required
[tool.pyfaro.dev]
preset = "fastapi"
```

```toml
# invalid: tool.pyfaro.dev.entry-point must be a module name
[tool.pyfaro.dev]
entry-point = "demo.main:app"
```

```toml
# invalid: tool.pyfaro.dev.reload requires preset = "fastapi"
[tool.pyfaro.dev]
entry-point = "demo.worker"
reload = true
```

## `[tool.pyfaro.tasks]`

Named commands for [`pyfaro run <task>`](../commands/run.md#named-tasks). The
table must be a table (`tool.pyfaro.tasks must be a table`) and each entry is a
task in one of two forms.

**An array of strings** is a command: the program and its arguments, started
without a shell.

```toml
[tool.pyfaro.tasks]
lint-all = ["ruff", "check", "."]
```

**A table** has exactly one of `command` or `shell` plus optional settings:

| Key | Type | Default | Rule | Error |
| --- | --- | --- | --- | --- |
| *task name* | array of strings, or table | | At most 64 characters, matching `^[A-Za-z0-9][A-Za-z0-9_.-]*$`. Quote a name that has a dot in the TOML key. | `invalid task name "<name>" in tool.pyfaro.tasks: use at most 64 characters, starting with a letter or digit, then letters, digits, '_', '.' or '-'`, `tool.pyfaro.tasks.<name> must be an array of strings or a table with command or shell` |
| `command` | array of strings | | Non-empty, every string non-empty and without NUL. The program and its arguments, no shell. Exactly one of `command` and `shell`. | `….command must be an array of non-empty strings`, `… must not be empty`, `… must not contain a NUL character` |
| `shell` | string | | Non-empty script, run with `sh -c` (Unix) or `cmd.exe /d /s /c` (Windows). An explicit opt-in; it takes no extra arguments from the command line. | `….shell must be a string`, `….shell must not be empty` |
| `cwd` | string | the project root | Relative to the project root, never absolute and never escaping it. Must exist as a directory when the task runs. | `….cwd must be relative to the project root, not absolute: <value>`, `….cwd must stay inside the project root: <value>`, `….cwd must not be empty` |
| `env` | table of strings | `{}` | Variables added to the inherited environment; task values win. No interpolation. Names are not empty and contain no `=` or NUL. | `….env must be a table of strings`, `….env.<name> must be a string`, `invalid variable name "<name>" in ….env: names must not be empty or contain '=' or NUL` |
| `description` | string | the command line | Shown when `pyfaro run` lists the tasks. | `….description must be a string` |

A task with both or neither of `command` and `shell` fails with
`tool.pyfaro.tasks.<name> must set exactly one of command or shell`, and an
unknown key with `unknown task setting: tool.pyfaro.tasks.<name>.<key>
(supported: command, shell, cwd, env, description)`.

Checks that depend on the machine happen when the task is launched, before
uv starts: the `cwd` directory must exist and stay inside the project
(`task "<name>": cwd "<value>" is not an existing directory …`), a shell task
rejects extra arguments (`task "<name>" is a shell task and does not accept
extra arguments …`), and on Windows a shell script containing a double quote
is rejected.

```toml
[tool.pyfaro.tasks]
lint-all = ["ruff", "check", "."]
docs = { command = ["python", "-m", "http.server", "8001"], cwd = "docs", env = { PYTHONUNBUFFERED = "1" }, description = "Serve the docs folder" }
changes = { shell = "git log --oneline | head -n 5" }
"build.docs" = ["mkdocs", "build"]
```

Mistakes:

```toml
# invalid: must set exactly one of command or shell
[tool.pyfaro.tasks]
both = { command = ["ruff", "check"], shell = "ruff check" }
```

```toml
# invalid: tool.pyfaro.tasks.serve.cwd must be relative to the project root
[tool.pyfaro.tasks]
serve = { command = ["python", "-m", "http.server"], cwd = "/srv/www" }
```

```toml
# invalid: unknown task setting: tool.pyfaro.tasks.lint.args
[tool.pyfaro.tasks]
lint = { command = ["ruff", "check"], args = ["."] }
```

```toml
# invalid: invalid task name
[tool.pyfaro.tasks]
"-bad" = ["ruff", "check"]
```

The semantics of running a task, including how a task name differs from a
program name, are in the [`run` guide](../commands/run.md#named-tasks).

## Settings Pyfaro does not own

Pyfaro does not read, interpret or rewrite `[tool.ruff]`, `[tool.ty]`,
`[tool.pytest]`, `[tool.uv]` or any other section, and it adds no path-valued
setting in schema version `1`: uv and the tools keep ownership of their own
configuration. `pyfaro init --apply` keeps your comments and formatting when it
adds its table. `pyfaro doctor` warns when a tool has settings in more than one
place.

## See also

- [Configuration](../concepts/configuration.md): the concepts.
- [Environment variables](environment-variables.md).
- [Command line](cli.md).
