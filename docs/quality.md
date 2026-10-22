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

# Type checking

`pyfaro type` runs [ty](https://docs.astral.sh/ty/) on a project.

```sh
pyfaro type [PATHS]... [--project <PATH>]   # ty check
```

## Contract

- Runs `ty check` directly with the project root as its working directory.
  Diagnostics stream straight through unchanged, and ty's exit status is
  returned unchanged (0 clean, 1 diagnostics, 2 ty errors).
- When the project environment exists (`.venv`, or `UV_PROJECT_ENVIRONMENT`),
  Pyfaro passes `--python <environment>` so ty resolves the project's installed
  dependencies even if `UV_PROJECT_ENVIRONMENT` points elsewhere. When it does
  not exist, ty uses its own discovery. Pyfaro never creates it.
- Positional paths are relative to the directory where you ran `pyfaro`.
  Pyfaro makes them absolute and, when paths are given, adds
  `--force-exclude` so project exclusions still apply.
- Configuration comes from `[tool.ty]` in `pyproject.toml` or `ty.toml`.
  Pyfaro passes no configuration overrides.
- ty is found like Ruff. If it is missing, the command exits 1 and tells you to
  run `pyfaro add --dev ty` then `pyfaro sync`.

## Limitations

- ty is pre-1.0: its diagnostics, rules and flags may change between releases.
- Analysis only sees what the environment has installed. Run `pyfaro sync`
  first; `pyfaro type` never syncs implicitly.

## Tested version

Tested with ty 0.0.85. Other versions are not rejected.
