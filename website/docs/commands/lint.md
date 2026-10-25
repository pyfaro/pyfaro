---
description: "Lint Python code with Ruff through pyfaro lint, inside the project environment, with Ruff's output passed through."
---

# pyfaro lint

`pyfaro lint` runs [Ruff](https://docs.astral.sh/ruff/)'s linter (`ruff check`)
on your project. Use it to catch unused imports, import-order problems and
other rule violations before review or CI.

Pyfaro does not reimplement Ruff or interpret its configuration: it finds the
right Ruff, runs it from the project root and returns its exit code.

## Quick example

```console
$ pyfaro lint
All checks passed!
```

```text
pyfaro lint [PATHS]... [--project <PATH>]
```

## Scenarios

The examples use the project from `pyfaro create lib demo` plus two messy
files, `src/demo/messy.py` and `scripts/tool.py`.

### Lint the whole project

```console
$ pyfaro lint
I001 [*] Import block is un-sorted or un-formatted
 --> scripts/tool.py:1:1
  |
1 | import json
  | ^^^^^^^^^^^
2 | x=1
  |
help: Organize imports
  |
1 | import json
2 +
3 | x=1
  |

F401 [*] `json` imported but unused
 --> scripts/tool.py:1:8
  |
1 | import json
  |        ^^^^
2 | x=1
  |
help: Remove unused import: `json`
  |
  - import json
1 | x=1
  |

F401 [*] `os` imported but unused
 --> src/demo/messy.py:1:8
  |
1 | import os
  |        ^^
2 | import sys
  |
help: Remove unused import: `os`
  |
  - import os
1 | import sys
  |

F401 [*] `sys` imported but unused
 --> src/demo/messy.py:2:8
  |
1 | import os
2 | import sys
  |        ^^^
help: Remove unused import: `sys`
  |
1 | import os
  - import sys
2 |
  |

Found 4 errors.
[*] 4 fixable with the `--fix` option.
error: ruff failed (exit status: 1); see ruff's output above
```

Ruff's report is untouched. The last line is Pyfaro's: Ruff exited with `1`,
and so does `pyfaro lint`.

!!! note
    `pyfaro lint` does not take `--fix`. Ruff's output says some findings are
    fixable; apply them with [`pyfaro check --fix`](check.md), which also
    formats and re-verifies.

### Lint specific paths

Pass files or directories after the command. They are relative to **the
directory where you ran `pyfaro`**, not to the project root, so tab completion
works as in any shell:

```console
$ pyfaro lint src
…
Found 2 errors.
[*] 2 fixable with the `--fix` option.
error: ruff failed (exit status: 1); see ruff's output above
```

From inside a subdirectory, the same file is reachable by the path you would
use for `ls`; the reports always show project-relative paths:

```console
$ cd src/demo
$ pyfaro lint messy.py
F401 [*] `os` imported but unused
 --> src/demo/messy.py:1:8
…
```

A path that does not exist is a Ruff error:

```console
$ pyfaro lint nonexistent.py
E902 No such file or directory (os error 2)
--> nonexistent.py:1:1

Found 1 error.
error: ruff failed (exit status: 1); see ruff's output above
```

### Lint another project: `--project`

`--project <PATH>` selects exactly that project directory instead of searching
upward from the current one. The `PATHS` stay relative to your current
directory:

```console
$ pyfaro lint --project demo demo/src
All checks passed!
```

Here `demo/src` is resolved from the directory you are in, not from `demo`.
`--project` may appear before or after the paths. The project configuration
must load before Ruff starts.

## Path handling

- Positional paths are relative to your current directory. Pyfaro turns them
  into absolute paths before starting Ruff.
- Ruff itself runs with the **project root** as its working directory.
- With no paths, Ruff checks the whole project root.
- With explicit paths, Pyfaro adds `--force-exclude`, so a file your
  configuration excludes stays excluded even when you name it.

## Configuration

Ruff reads its own configuration: `[tool.ruff]` in `pyproject.toml`,
`ruff.toml` or `.ruff.toml`. Pyfaro never passes overrides. A new project from
`pyfaro create` contains:

```toml
[tool.ruff]
line-length = 88
src = ["src", "tests"]
```

To exclude a directory, use Ruff's own setting:

```toml
[tool.ruff]
extend-exclude = ["scripts"]
```

With that, `scripts/tool.py` no longer appears in the whole-project run, and
naming it explicitly does not bring it back:

```console
$ pyfaro lint scripts/tool.py
warning: No Python files found under the given path(s)
All checks passed!
```

!!! warning
    Keep one source of Ruff configuration. `pyfaro doctor` reports
    `[tool.ruff]` next to a `ruff.toml` as an error.

## Guarantees

- **No implicit sync.** `pyfaro lint` never creates or modifies `.venv` or
  `uv.lock`. Install tools with `pyfaro add --dev ruff` and `pyfaro sync`.
- **No source writes.** It only reads your code. Ruff may refresh its own
  cache (`.ruff_cache`).
- Ruff runs directly (no shell, no `uv run`), output streams through, stdin is
  inherited, and cancelling stops Ruff's process tree.

## Errors and fixes

A problem in Ruff's configuration is reported by Ruff, with exit code `2`:

```console
$ pyfaro lint
ruff failed
  Cause: Unknown rule selector `NOPE999` in `select` from `~/demo/pyproject.toml`
error: ruff failed (exit status: 2); see ruff's output above
```

Ruff is looked up in the project environment, then `VIRTUAL_ENV`, then `PATH`.
In a project that was not synced:

```console
$ pyfaro lint
error: ruff: not found in the project environment, VIRTUAL_ENV, or PATH; run `pyfaro add --dev ruff` then `pyfaro sync`
```

Run exactly those two commands (the first only if Ruff is not already in your
`dev` group). Outside any project:

```console
$ cd /tmp && pyfaro lint
error: no pyproject.toml found from /tmp; use --project <PATH>
```

## Exit codes

| Code | Meaning |
| --- | --- |
| `0` | No violations |
| `1` | Violations found, **or** a Pyfaro failure (no project, Ruff missing) |
| `2` | Ruff itself failed, for example on an unknown rule selector in `[tool.ruff]` (Ruff's exit status is returned unchanged) |
| `130` | Cancelled |

## See also

- [`pyfaro fmt`](fmt.md) — formatting with Ruff.
- [`pyfaro check`](check.md) — format check, lint and type check together, with `--fix`.
- [`pyfaro tools`](tools.md) — see which Ruff is used.
