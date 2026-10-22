# Linting and formatting

`pyfaro lint` and `pyfaro fmt` run [Ruff](https://docs.astral.sh/ruff/) on a
project. Pyfaro does not reimplement Ruff or interpret its configuration.

```sh
pyfaro lint [PATHS]... [--project <PATH>]            # ruff check
pyfaro fmt [PATHS]... [--check] [--project <PATH>]   # ruff format [--check]
```

## Contract

- `lint` runs `ruff check`. `fmt` runs `ruff format` and rewrites files;
  `fmt --check` runs `ruff format --check`, which never modifies files.
- Ruff reads the project's own configuration (`[tool.ruff]` in
  `pyproject.toml`, `ruff.toml`, `.ruff.toml`). Pyfaro never passes Ruff
  configuration overrides, so exclusions and rule settings apply unchanged.
- Ruff runs directly (no shell, no `uv run`) with the project root as its
  working directory. Output streams straight through, stdin is inherited,
  and cancellation stops the Ruff process tree and exits 130.

## Project selection

Without `--project`, Pyfaro finds the nearest `pyproject.toml` from the current
directory upward. `--project <PATH>` selects exactly that directory. The
project configuration must load before Ruff starts.

## Paths

Positional paths are relative to the directory where you ran `pyfaro`, not to
the project root. Pyfaro makes them absolute before Ruff starts. Without paths,
Ruff checks the project root. With explicit paths Pyfaro adds
`--force-exclude`, so files that the project excludes stay excluded even when
named explicitly.

## Finding Ruff

Pyfaro resolves `ruff` in this order, as `pyfaro tools` does: the project
environment (`.venv`, or `UV_PROJECT_ENVIRONMENT`), then `VIRTUAL_ENV`, then
`PATH`. If Ruff is missing, the command exits 1 with:

```text
error: ruff: not found in the project environment, VIRTUAL_ENV, or PATH; run `pyfaro add --dev ruff` then `pyfaro sync`
```

## Exit codes

Ruff's numeric exit status is returned unchanged (for example 1 for lint
violations or files that `fmt --check` would reformat, 2 for Ruff errors).
Pyfaro's own failures, such as no project or a missing Ruff, exit 1;
cancellation exits 130.

## No implicit sync

These commands never create or modify `.venv` or `uv.lock`. Install tools
explicitly with `pyfaro add --dev ruff` and `pyfaro sync`.

## Tested version

Tested with Ruff 0.16.10. Other versions are not rejected.
