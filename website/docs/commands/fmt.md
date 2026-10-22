# pyfaro fmt

`pyfaro fmt` formats your code with Ruff (`ruff format`). Use it to rewrite
files in place before committing, or with `--check` to verify formatting
without touching anything (the form CI uses).

## Quick example

```console
$ pyfaro fmt --check
3 files already formatted
```

```text
pyfaro fmt [PATHS]... [--check] [--project <PATH>]
```

## Scenarios

The examples use `pyfaro create lib demo` plus two unformatted files,
`src/demo/messy.py` and `scripts/tool.py`.

### See what would change: `--check`

`--check` runs `ruff format --check`. It prints a diff for each file that would
change and **never modifies a file**:

```console
$ pyfaro fmt --check
unformatted: File would be reformatted
 --> scripts/tool.py:2:1
  |
1 | import json
  - x=1
2 +
3 + x = 1
  |

unformatted: File would be reformatted
  --> src/demo/messy.py:5:11
  |
4 |
  - def add(a:int,b:int)->int:
  -     return a+b
5 + def add(a: int, b: int) -> int:
6 +     return a + b
7 |

2 files would be reformatted, 3 files already formatted
error: ruff failed (exit status: 1); see ruff's output above
```

Exit code `1` means "some files would change".

### Format only some paths

Paths are relative to the directory where you ran `pyfaro`. Here only `src` is
formatted, so `scripts/tool.py` is untouched:

```console
$ pyfaro fmt src
1 file reformatted, 1 file left unchanged
$ head -2 scripts/tool.py
import json
x=1
```

`--check` accepts paths as well: `pyfaro fmt --check src/demo/messy.py`.

### Format the project

Without paths, everything under the project root is formatted. Continuing
from above, `scripts/tool.py` is the one file left to reformat:

```console
$ pyfaro fmt
1 file reformatted, 4 files left unchanged
$ pyfaro fmt --check
5 files already formatted
```

### Another project: `--project`

`--project <PATH>` selects exactly that project instead of the nearest
`pyproject.toml` above the current directory. Paths stay relative to your
current directory.

## Path handling

- Positional paths are relative to your current directory; Pyfaro makes them
  absolute before starting Ruff.
- Ruff runs with the project root as its working directory.
- With explicit paths Pyfaro adds `--force-exclude`, so files excluded by your
  Ruff configuration stay excluded even if you name them. With
  `extend-exclude = ["scripts"]` (see below):

```console
$ pyfaro fmt --check scripts/tool.py
warning: No Python files found under the given path(s)
```

## Configuration

Ruff owns the configuration: `[tool.ruff]` in `pyproject.toml`, `ruff.toml` or
`.ruff.toml` (formatter options live under `[tool.ruff.format]`). Pyfaro passes
no overrides. A generated project has `line-length = 88` and
`src = ["src", "tests"]`. Exclusions use Ruff's `exclude` and `extend-exclude`:

```toml
[tool.ruff]
extend-exclude = ["scripts"]
```

`pyfaro doctor` flags a second Ruff configuration source next to
`[tool.ruff]`.

## Guarantees

- **No implicit sync.** `.venv` and `uv.lock` are never created or modified.
- **Writes only without `--check`.** Plain `pyfaro fmt` rewrites source files;
  `--check` does not. Ruff may refresh its own `.ruff_cache`.

!!! tip
    [`pyfaro check --fix`](check.md) already formats after applying lint
    fixes, so you do not need to run `pyfaro fmt` after it.

## Errors and fixes

```console
$ pyfaro fmt
error: ruff: not found in the project environment, VIRTUAL_ENV, or PATH; run `pyfaro add --dev ruff` then `pyfaro sync`
```

Run `pyfaro add --dev ruff` (skip if it is already in your `dev` group), then
`pyfaro sync`. Outside a project: `error: no pyproject.toml found from …; use
--project <PATH>`.

## Exit codes

| Code | Meaning |
| --- | --- |
| `0` | Formatted, or nothing to change with `--check` |
| `1` | `--check` found files that would change, **or** a Pyfaro failure (no project, Ruff missing) |
| `2` | Ruff itself failed (its exit status is returned unchanged) |
| `130` | Cancelled |

## See also

- [`pyfaro lint`](lint.md) — rule checks.
- [`pyfaro check`](check.md) — format check, lint and type check in one run.
