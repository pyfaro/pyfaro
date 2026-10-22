# pyfaro sync

`pyfaro sync` makes the project's virtual environment match its dependencies,
by delegating to `uv sync`. Use it after cloning a project, after changing
dependencies, and before `check` or `test`, since those commands never sync
implicitly. uv owns resolution, `uv.lock`, `.venv` creation and installs.

## Quick example

```console
$ pyfaro sync
Resolved 9 packages in 8ms
   Building acme-widget @ file://.
      Built acme-widget @ file://.
Prepared 1 package in 3ms
Installed 8 packages in 11ms
 + acme-widget==0.1.0 (from file://.)
 + iniconfig==2.3.1
 + packaging==26.3
 + pluggy==1.7.0
 + pygments==2.21.0
 + pytest==9.1.1
 + ruff==0.16.10
 + ty==0.0.85
```

## Scenarios

### 1. Default sync

With no flag, uv may update `uv.lock` before installing. This is the mode for
a first sync, which creates `.venv` and `uv.lock`.

### 2. `--locked`: fail if the lock is stale

`--locked` requires a present, up-to-date lock and refuses to modify it. This
is the right mode for CI and for everyday use once `uv.lock` is committed.

```console
$ pyfaro sync --locked
Resolved 10 packages in 0.60ms
Audited 8 packages in 0.13ms
```

If `pyproject.toml` declares a dependency the lock does not know about:

```console
$ pyfaro sync --locked
Resolved 11 packages in 99ms
The lockfile at `uv.lock` needs to be updated, but `--locked` was provided. To update the lockfile, run `uv lock`.
error: uv sync failed (exit status: 1); see uv's output above
```

Fix it by running `pyfaro sync` (without the flag), then commit the lock.

!!! note
    In a project with no `uv.lock` at all, `--locked` fails with
    `Unable to find lockfile at uv.lock`. uv may already have created `.venv`
    by then.

### 3. `--frozen`: trust the lock as it is

`--frozen` requires an existing lock and **skips the freshness check**. Newly
declared dependencies are *not* installed:

```console
$ pyfaro sync --frozen
   Building acme-widget @ file://.
      Built acme-widget @ file://.
Installed 1 package in 0.27ms
 ~ acme-widget==0.1.0 (from file://.)
$ .venv/bin/python -c "import idna"
ModuleNotFoundError: No module named 'idna'
```

Use it only when you deliberately want the lock to win. `--locked` and
`--frozen` are mutually exclusive:

```console
$ pyfaro sync --locked --frozen
error: the argument '--locked' cannot be used with '--frozen'
```

### 4. `--offline`

`--offline` disables uv's network access. It works when the needed packages
are in uv's cache:

```console
$ pyfaro sync --offline
Resolved 11 packages in 8ms
Installed 1 package in 3ms
 + idna==3.20
```

### 5. Choose uv with `--uv`

`--uv <path>` selects the uv executable (default `uv`). uv 0.9.26 or newer
(stable) is required, and 0.9.26 is the tested version.

## Project selection

Without options, Pyfaro looks for the nearest `pyproject.toml` in the current
directory and its ancestors, so `pyfaro sync` works from any subdirectory.

| Form | Meaning |
| --- | --- |
| `pyfaro sync` | Discover upward from the current directory. |
| `pyfaro sync ../acme-widget` | Start the same upward search from `PATH`. |
| `pyfaro sync --project ../acme-widget` | Use exactly that directory. It must hold a `pyproject.toml`. It never falls back to a parent. |

```console
$ cd other        # a directory with no pyproject.toml
$ pyfaro sync
error: no pyproject.toml found from .; use --project <PATH>
$ pyfaro sync --project ../acme-widget --locked
Resolved 11 packages in 0.55ms
Audited 10 packages in 0.13ms
$ pyfaro sync --project ../acme-widget/src --locked
error: project directory has no pyproject.toml: …/acme-widget/src
```

Symlinks are resolved first, so a linked working directory selects its
physical project.

## What it writes, and what it never writes

**Writes** (through uv): `.venv` and `uv.lock` of the selected project, and
the installed packages. An exact sync may **remove** packages that are no
longer declared.

**Never writes:** a second lockfile, or `pyproject.toml`. Pyfaro does not
resolve dependencies itself, and it passes `--no-python-downloads`, so it
never downloads an interpreter. A compatible Python must already be
installed. Use [`pyfaro python install`](python.md) to get one.

## Errors you may hit

| Message | Fix |
| --- | --- |
| `no pyproject.toml found from …; use --project <PATH>` | `cd` into a project, or pass `--project`. |
| `The lockfile at uv.lock needs to be updated, but --locked was provided` | Run `pyfaro sync` without `--locked` and commit `uv.lock`. |
| `Unable to find lockfile at uv.lock` | Run `pyfaro sync` once without `--locked` or `--frozen`. |
| `No interpreter found …` | Install a compatible Python, for example `pyfaro python install 3.12`. |
| `the argument '--locked' cannot be used with '--frozen'` | Pick one. |

## Exit codes

| Code | Meaning |
| --- | --- |
| `0` | The environment is in sync. |
| `1` | A Pyfaro error (no project found), or uv failed with status 1. |
| `2` | A usage error, or uv failed with status 2. |
| other | uv's failure status is forwarded after `uv sync failed (exit status: N)`. |
| `130` | Cancelled. |

## See also

- [`pyfaro add` and `remove`](add-remove.md): change dependencies without syncing.
- [`pyfaro run`](run.md): run a command in the synced environment.
- [`pyfaro init`](init.md): create the lock for an existing project.
