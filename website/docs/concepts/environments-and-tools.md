# Environments and tools

## The project environment

Each project has one virtual environment, created and managed by uv. By
default it is `.venv` in the project root. `pyfaro sync` and `pyfaro run`
bring it up to date; `add` and `remove` only change `pyproject.toml` and
`uv.lock`.

### `UV_PROJECT_ENVIRONMENT`

Set `UV_PROJECT_ENVIRONMENT` to use another location, as uv does:

- an absolute path is used as is;
- a relative path is resolved from the **project root**, not from where you
  ran the command.

```console
$ UV_PROJECT_ENVIRONMENT=.envs/dev pyfaro sync
$ UV_PROJECT_ENVIRONMENT=.envs/dev pyfaro tools ruff
ruff: ~/code/greeter/.envs/dev/bin/ruff (ruff 0.16.10)
```

Set it for every command, or the commands will look in `.venv`.

## How tools are found

Ruff, ty and pytest are looked up in this order, and the first match wins:

1. **The project environment**: `.venv`, or `UV_PROJECT_ENVIRONMENT`.
2. **`VIRTUAL_ENV`**: the environment you activated, if any.
3. **`PATH`**.

uv is different: it comes from `PATH`, unless you pass `--uv <PATH>`, and is
validated against the minimum version (0.9.26).

```console
$ pyfaro tools
uv: ~/.local/bin/uv (uv 0.9.26)
ruff: ~/code/greeter/.venv/bin/ruff (ruff 0.16.10)
ty: ~/code/greeter/.venv/bin/ty (ty 0.0.85)
pytest: ~/code/greeter/.venv/bin/pytest (pytest 9.1.1)
```

!!! tip "Pin the tools in the project"
    Projects made by `pyfaro create` list Ruff, ty and pytest in the `dev`
    dependency group. Everybody who runs `pyfaro sync` then gets the same
    versions, and Pyfaro finds them in step 1. Add them to an existing
    project with `pyfaro add --dev ruff ty pytest`.

### Nothing is installed for you

Quality commands never sync. If a tool is missing, the command fails and
tells you what to do:

```console
$ pyfaro lint
error: ruff: not found in the project environment, VIRTUAL_ENV, or PATH; run `pyfaro add --dev ruff` then `pyfaro sync`
```

Likewise `pyfaro test` needs pytest *inside* the project environment, so
that it imports your project's packages. It reports an error pointing at
`pyfaro sync` if that environment does not exist yet:

```console
$ pyfaro test
error: pytest: project environment interpreter not found at ~/code/greeter/.venv/bin/python; run `pyfaro sync` first
```

## Python interpreters

Pyfaro disables uv's automatic Python downloads. An interpreter must already
be installed, or you ask for one explicitly:

```console
$ pyfaro python install 3.13
$ pyfaro python pin 3.13
```

See [`pyfaro python`](../commands/python.md) and the
[Python versions](../workflows/python-versions.md) workflow.

## Next steps

- [`pyfaro tools`](../commands/tools.md) and [`pyfaro doctor`](../commands/doctor.md).
- [Environment variables](../reference/environment-variables.md).
- [Troubleshooting](../workflows/troubleshooting.md).
