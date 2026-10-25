---
description: "What a Pyfaro project is, how it is found from the current directory, and why uv workspaces are not supported yet."
---

# Projects

A Pyfaro project is a directory with a `pyproject.toml` that has a standard
`[project]` table. Nothing else is required; `[tool.pyfaro]` is
[optional](configuration.md).

## Discovery

Most commands find the project by looking for the nearest `pyproject.toml`
in the current directory, then each parent directory. Symlinks are resolved
first, so a linked working directory selects its physical project.

```console
$ cd greeter/src/greeter
$ pyfaro check        # runs for ~/code/greeter
```

If no project is found, the command fails with exit code 1:

```console
$ cd /tmp
$ pyfaro sync
error: no pyproject.toml found from /tmp; use --project <PATH>
```

## `--project`

`--project <PATH>` selects an exact directory. It overrides discovery and
**never falls back to a parent directory**.

```console
$ pyfaro sync --project ./greeter --locked
$ pyfaro test --project ./greeter -k parser
```

A path that does not exist is an error:

```console
$ pyfaro sync --project /nonexistent
error: could not resolve project directory /nonexistent: No such file or directory (os error 2)
```

The commands differ slightly in how they start:

| Command | How the project is chosen |
| --- | --- |
| `sync` | Discovery from `[PATH]` (default: current directory), or `--project` |
| `run`, `add`, `remove` | Discovery from the current directory, or `--project` |
| `lint`, `fmt`, `type`, `check`, `test`, `tools`, `doctor` | Nearest project, or `--project` |
| `python` | Exact: `--project`, otherwise the current directory, with no discovery |
| `init` | The `[PATH]` argument (default: current directory) |
| `create` | None: it makes a new directory |

!!! note "Paths are relative to where you are"
    The `PATHS` you pass to `lint`, `fmt`, `type` and `check` are relative
    to your current directory, while the tools themselves run from the project
    root. `pyfaro test` arguments are passed to pytest as written, and its
    paths are relative to the project root.

!!! warning "Workspaces are not supported"
    Pyfaro works on standalone projects. `doctor` reports a uv workspace root
    or member as unsupported.

## Next steps

- [Configuration](configuration.md).
- [Environments and tools](environments-and-tools.md).
- The [`sync`](../commands/sync.md) and [`python`](../commands/python.md) pages.
