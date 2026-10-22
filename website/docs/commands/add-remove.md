# pyfaro add and remove

`pyfaro add` and `pyfaro remove` edit the project's dependencies through
`uv add` and `uv remove`. uv writes `pyproject.toml` and `uv.lock`. Use them
instead of editing the file by hand when you want uv to resolve the change.

!!! important "No implicit sync"
    Both commands pass `--no-sync`: they update the declaration and the lock,
    but they do **not** install anything into `.venv`. Run
    [`pyfaro sync`](sync.md) when you want the environment to follow.

## Quick example

```console
$ pyfaro add "tomli-w>=1,<2"
Resolved 10 packages in 81ms
$ pyfaro sync --locked
 + tomli-w==1.2.0
$ pyfaro remove tomli-w
Resolved 9 packages in 18ms
```

## Scenarios

### 1. Add a runtime dependency

`add` takes PEP 508 requirement specifiers, and several at once. Without
`--dev` or `--group`, they go to `[project].dependencies`:

```console
$ pyfaro add "requests>=2,<3" rich
```

### 2. `--dev`: the development group

```console
$ pyfaro add --dev pytest-cov
Resolved 12 packages in 238ms
```

The requirement lands in the `dev` dependency group:

```toml
[dependency-groups]
dev = [
    "pytest==9.1.1",
    "pytest-cov>=7.1.0",
    "ruff==0.16.10",
    "ty==0.0.85",
]
```

### 3. `--group`: a named group

```console
$ pyfaro add --group docs mkdocs
Resolved 28 packages in 180ms
```

`--dev` and `--group` are mutually exclusive:

```console
$ pyfaro add --dev --group docs mkdocs
error: the argument '--dev' cannot be used with '--group <GROUP>'
```

### 4. Prove there is no sync

```console
$ pyfaro add rich
Resolved 31 packages in 148ms
$ .venv/bin/python -c "import rich"
ModuleNotFoundError: No module named 'rich'
$ pyfaro sync
 + rich==…
```

### 5. Remove dependencies

`remove` takes dependency **names**, matching the section you used to add
them:

```console
$ pyfaro remove rich tomli-w
Resolved 27 packages in 18ms
$ pyfaro remove --dev pytest-cov
$ pyfaro remove --group docs mkdocs
```

Removing a name from the wrong section fails:

```console
$ pyfaro remove nothere
error: The dependency `nothere` could not be found in `project.dependencies`
error: uv remove failed (exit status: 2); see uv's output above
```

!!! note
    Removing the last entry of a group leaves an empty group (`docs = []`)
    in `pyproject.toml`. That is uv's behavior.

### 6. Requirements that start with a dash

Put `--` before them so Pyfaro does not parse them as options. Everything
after it reaches uv untouched, and uv decides whether it accepts it:

```console
$ pyfaro add -- "-e ./x"
error: Failed to parse: `-e ./x`
  Caused by: Expected package name starting with an alphanumeric character, found `-`
-e ./x
^
error: uv add failed (exit status: 2); see uv's output above
```

### 7. Offline, and choosing uv

`--offline` disables network access, and works when uv's cache can answer.
`--uv <path>` selects the uv binary. uv 0.9.26 or newer is required.

```console
$ pyfaro add --offline attrs
Resolved 10 packages in 9ms
```

## Project selection

Run the commands in a project or any of its subdirectories. Pyfaro uses the
nearest `pyproject.toml`. `--project <PATH>` selects an exact directory, which
must contain its own `pyproject.toml`:

```console
$ pyfaro add --project ./app --offline "local-package>=1"
```

Unlike `sync`, there is no positional path.

## What it writes, and what it never writes

**Writes** (through uv): the matching dependency table in `pyproject.toml`
(including `tool.uv.sources` when uv adds one) and `uv.lock`.

**Never writes:** `.venv` or installed packages. Pyfaro itself does not edit
`pyproject.toml` and does not resolve anything, since uv owns both.

## Errors you may hit

| Message | Fix |
| --- | --- |
| `No solution found when resolving dependencies` | uv could not satisfy the requirement. Read the explanation above the error. A typo in the name is the common cause. |
| `The dependency … could not be found in …` | Use the same `--dev` or `--group` you used to add it. |
| `Failed to parse: …` | Fix the requirement specifier. |
| `the following required arguments were not provided: <PACKAGES>...` | Pass at least one package. |
| `no pyproject.toml found …` | Run inside a project or pass `--project`. |

## Exit codes

| Code | Meaning |
| --- | --- |
| `0` | The dependency change was applied. |
| `1` | A Pyfaro error, or uv failed with status 1 (for example an unsatisfiable requirement). |
| `2` | A usage error, or uv failed with status 2 (for example an unknown dependency). |
| `130` | Cancelled. |

uv's numeric status is forwarded unchanged.

## See also

- [`pyfaro sync`](sync.md): install what you declared.
- [`pyfaro run`](run.md): run commands in the environment.
- [`pyfaro create`](create.md): a new project starts with pinned dev tools.
