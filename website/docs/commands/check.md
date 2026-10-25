---
description: "Run the whole quality gate with pyfaro check: format check, lint and type check in one run, with one exit code."
---

# pyfaro check

`pyfaro check` is the one command for "is my project clean?". It runs the
format check, the linter and the type checker, then prints a three-line
summary. Use it before committing and as the quality gate in CI.

## Quick example

```console
$ pyfaro check
3 files already formatted
All checks passed!
All checks passed!

== pyfaro check summary ==
format: ok
lint: ok
type: ok
check: ok
```

```text
pyfaro check [PATHS]... [--fix] [--project <PATH>]
```

## Steps

`check` runs these steps **in this order**, and always runs all of them, even
when an earlier one fails:

1. `format` — `ruff format --check`
2. `lint` — `ruff check`
3. `type` — `ty check`

Each tool's own output streams through as it runs, so reading from the top
shows format output first, then lint, then type. `PATHS` and `--project`
apply to every step.

## The summary

After the steps, `check` prints a blank line, a `== pyfaro check summary ==`
header and one line per step, to **stderr** (stdout carries only the tools'
output). Each step is `ok`, `failed (exit N)` with the tool's exit code, or
`unavailable (…)`. The last line is `check: ok`, or `check: failed (…)` listing
the failing steps.

## Scenarios

The examples use `pyfaro create lib demo` plus `src/demo/messy.py` and
`scripts/tool.py`: unformatted, with unused imports, and one type error.

### A failing project

```console
$ pyfaro check
…
== pyfaro check summary ==
format: failed (exit 1)
lint: failed (exit 1)
type: failed (exit 1)
check: failed (format, lint, type)
```

Fix everything the tools can fix with `--fix` (below), then repair the rest by
hand.

### Restrict to some paths

Paths are relative to the directory where you ran `pyfaro`:

```console
$ pyfaro check src/demo/messy.py
1 file already formatted
F401 [*] `os` imported but unused
…
Found 2 errors.
[*] 2 fixable with the `--fix` option.
error[invalid-return-type]: Return type does not match returned value
…
Found 1 diagnostic

== pyfaro check summary ==
format: ok
lint: failed (exit 1)
type: failed (exit 1)
check: failed (lint, type)
```

### Apply safe fixes first: `--fix`

`pyfaro check --fix` applies automatic fixes, then verifies. The sequence is:

1. Print which checks can be fixed: format and lint can; type cannot.
2. `ruff check --fix` — safe fixes only. Pyfaro never passes `--unsafe-fixes`.
3. `ruff format` — rewrites files. Formatting comes **last** so that lint
   fixes cannot leave unformatted code behind.
4. Re-run the full non-writing verification and print the summary.

```console
$ pyfaro check --fix
fix: format and lint can be fixed automatically; type has no automatic fixes
Found 3 errors (3 fixed, 0 remaining).
fix lint: ok
2 files reformatted, 3 files left unchanged
fix format: ok
fix: rechecking without writing
5 files already formatted
All checks passed!
error[invalid-return-type]: Return type does not match returned value
 --> src/demo/messy.py:6:12
  |
5 | def label(n: int) -> str:
  |                      --- Expected `str` because of return type
6 |     return n
  |            ^ expected `str`, found `int`

Found 1 diagnostic

== pyfaro check summary ==
format: ok
lint: ok
type: failed (exit 1)
check: failed (type)
```

The unused imports are gone and the code is formatted; only the type error,
which needs a human, remains. A failure in a fix step (`fix lint: failed
(exit 2)`) is reported but does not stop the next step. The exit code follows
the final verification.

### Another project: `--project`

`--project <PATH>` selects the project directory explicitly; otherwise the
nearest `pyproject.toml` upward is used.

## Unavailable steps

If a tool is not installed, only its steps are marked `unavailable`, count as
failures, and the others still run. In a project that was never synced:

```console
$ pyfaro check

== pyfaro check summary ==
format: unavailable (ruff not found in the project environment, VIRTUAL_ENV, or PATH; run `pyfaro add --dev ruff` then `pyfaro sync`)
lint: unavailable (ruff not found in the project environment, VIRTUAL_ENV, or PATH; run `pyfaro add --dev ruff` then `pyfaro sync`)
type: unavailable (ty not found in the project environment, VIRTUAL_ENV, or PATH; run `pyfaro add --dev ty` then `pyfaro sync`)
check: failed (format, lint, type)
```

With `--fix` the fix steps are `unavailable` the same way, and the recheck
follows:

```console
$ pyfaro check --fix
fix: format and lint can be fixed automatically; type has no automatic fixes
fix lint: unavailable (ruff not found in the project environment, VIRTUAL_ENV, or PATH; run `pyfaro add --dev ruff` then `pyfaro sync`)
fix format: unavailable (ruff not found in the project environment, VIRTUAL_ENV, or PATH; run `pyfaro add --dev ruff` then `pyfaro sync`)
fix: rechecking without writing
…
```

## Path handling

Same as the individual commands: paths are relative to your current
directory, tools run from the project root, and with explicit paths
`--force-exclude` keeps your configured exclusions in force.

## Configuration

`check` has none of its own. Ruff reads `[tool.ruff]` (or `ruff.toml`) and ty
reads `[tool.ty]` (or `ty.toml`); see [`lint`](lint.md), [`fmt`](fmt.md) and
[`type`](type.md) for exclusions. Pyfaro passes no overrides.

## Guarantees

- **No implicit sync.** `.venv` and `uv.lock` are never created or modified.
- **No writes without `--fix`.** Plain `check` passes `--check` to
  `ruff format`, never passes `--fix` to `ruff check`, and `ty check` does not
  write. The tools may refresh their own caches (such as `.ruff_cache`).
- With `--fix`, source files can be rewritten. Review the result with
  `git diff`.

## Exit codes

| Code | Meaning |
| --- | --- |
| `0` | Every step passed |
| `1` | Any step failed or was unavailable, **or** a Pyfaro failure (no project) |
| `130` | Cancelled |

The tools' own exit codes appear in the summary, not in `check`'s exit code.

## See also

- [`pyfaro lint`](lint.md), [`pyfaro fmt`](fmt.md), [`pyfaro type`](type.md) — the
  individual steps.
- [`pyfaro test`](test.md) — run the tests (not part of `check`).
- [`pyfaro doctor`](doctor.md) — diagnose a missing tool.
