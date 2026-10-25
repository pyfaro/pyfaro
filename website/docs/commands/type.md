---
description: "Type-check a Python project with ty through pyfaro type."
---

# pyfaro type

`pyfaro type` type-checks your project with [ty](https://docs.astral.sh/ty/)
(`ty check`). Use it to catch mismatched types and bad calls before running
the code. ty is pre-1.0, so its diagnostics and rules may change between
releases.

## Quick example

```console
$ pyfaro type
All checks passed!
```

```text
pyfaro type [PATHS]... [--project <PATH>]
```

## Scenarios

The examples use `pyfaro create lib demo` plus `src/demo/messy.py`, which
returns an `int` from a function declared to return `str`.

### Check the whole project

```console
$ pyfaro type
error[invalid-return-type]: Return type does not match returned value
  --> src/demo/messy.py:10:12
   |
 9 | def label(n: int) -> str:
   |                      --- Expected `str` because of return type
10 |     return n
   |            ^ expected `str`, found `int`

Found 1 diagnostic
error: ty failed (exit status: 1); see ty's output above
```

ty's report is passed through unchanged; the last line is Pyfaro's.

### Check specific paths

Paths are relative to the directory where you ran `pyfaro`:

```console
$ pyfaro type src/demo/messy.py
error[invalid-return-type]: Return type does not match returned value
  --> src/demo/messy.py:10:12
…
Found 1 diagnostic
error: ty failed (exit status: 1); see ty's output above
```

Naming a file that the project excludes is not an error (see below).

### Another project: `--project`

`--project <PATH>` selects exactly that project directory instead of searching
upward from the current one. Paths remain relative to your current directory.

## Path handling

- Positional paths are relative to your current directory and made absolute
  before ty starts.
- ty runs with the project root as its working directory.
- With explicit paths Pyfaro adds `--force-exclude`, so project exclusions
  still apply.

## The environment

When the project environment exists (`.venv`, or `UV_PROJECT_ENVIRONMENT`),
Pyfaro passes `--python <environment>` to ty, so ty resolves your installed
dependencies. When it does not exist, ty uses its own discovery; Pyfaro never
creates it. Analysis only sees what is installed, so run `pyfaro sync` first.

## Configuration

ty reads `[tool.ty]` in `pyproject.toml`, or `ty.toml`. Pyfaro passes no
overrides. A generated project has no `[tool.ty]` section; ty's defaults apply.
To exclude a file:

```toml
[tool.ty.src]
exclude = ["src/demo/messy.py"]
```

Now the whole-project run passes, and so does naming the file:

```console
$ pyfaro type
All checks passed!
$ pyfaro type src/demo/messy.py
WARN No python files found under the given path(s)
All checks passed!
```

!!! warning
    Do not keep both `[tool.ty]` and a `ty.toml`; `pyfaro doctor` reports that
    as an error.

## Guarantees

- **No implicit sync.** `.venv` and `uv.lock` are never created or modified.
- **No source writes.** ty only reads. There is no automatic fix for type
  errors.

## Errors and fixes

```console
$ pyfaro type
error: ty: not found in the project environment, VIRTUAL_ENV, or PATH; run `pyfaro add --dev ty` then `pyfaro sync`
```

Run `pyfaro add --dev ty` (skip if it is already in your `dev` group), then
`pyfaro sync`. Outside a project you get `error: no pyproject.toml found from
…; use --project <PATH>`.

## Exit codes

| Code | Meaning |
| --- | --- |
| `0` | No diagnostics |
| `1` | Diagnostics found, **or** a Pyfaro failure (no project, ty missing) |
| `2` | ty itself failed (its exit status is returned unchanged) |
| `130` | Cancelled |

## See also

- [`pyfaro check`](check.md) — type check together with format and lint.
- [`pyfaro lint`](lint.md) and [`pyfaro fmt`](fmt.md).
- [`pyfaro tools`](tools.md) — see which ty is used.
