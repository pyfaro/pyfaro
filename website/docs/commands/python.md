---
description: "List, install and pin Python versions through uv with pyfaro python."
---

# pyfaro python

`pyfaro python` lists, installs and pins Python versions through uv. Use it to
see which interpreters are available, to fetch one explicitly, and to record
the version a project should use in `.python-version`. It is the **only**
Pyfaro command allowed to make uv download an interpreter.

## Quick example

```console
$ pyfaro python list --only-installed
cpython-3.14.8-linux-x86_64-gnu     /usr/bin/python3.14
cpython-3.13.11-linux-x86_64-gnu    ~/.local/share/uv/python/cpython-3.13.11-linux-x86_64-gnu/bin/python3.13
cpython-3.12.12-linux-x86_64-gnu    ~/.local/share/uv/python/cpython-3.12.12-linux-x86_64-gnu/bin/python3.12
$ pyfaro python pin 3.13
Pinned `.python-version` to `3.13`
```

## Scenarios

### 1. `list`: what is available

```console
$ pyfaro python list 3.12
cpython-3.12.12-linux-x86_64-gnu    ~/.local/share/uv/python/cpython-3.12.12-linux-x86_64-gnu/bin/python3.12
graalpy-3.12.0-linux-x86_64-gnu     <download available>
```

| Option | Effect |
| --- | --- |
| `[REQUEST]` | Filter by a Python request such as `3.12`. |
| `--only-installed` | Hide downloads that are only available. |
| `--all-versions` | Include old patch versions. |

```console
$ pyfaro python list --all-versions 3.12
cpython-3.12.12-linux-x86_64-gnu    ~/.local/share/uv/python/cpython-3.12.12-linux-x86_64-gnu/bin/python3.12
cpython-3.12.11-linux-x86_64-gnu    <download available>
cpython-3.12.10-linux-x86_64-gnu    <download available>
…
```

`list` is a catalog query. It installs nothing, and it is not filtered by the
project's `requires-python`.

### 2. `install`: fetch an interpreter explicitly

```console
$ pyfaro python install 3.13
Installed Python 3.13.11 in 31ms
 + cpython-3.13.11-linux-x86_64-gnu (python3.13)
$ pyfaro python install 3.12 3.13
All requested versions already installed
```

With no target, uv uses `.python-version` (or `.python-versions`) if present,
and otherwise applies its own default:

```console
$ pyfaro python install
Installed Python 3.12.12 in 32ms
 + cpython-3.12.12-linux-x86_64-gnu (python3.12)
```

An unknown request fails:

```console
$ pyfaro python install 3.99
error: No download found for request: cpython-3.99-linux-x86_64-gnu
error: uv python install failed (exit status: 2); see uv's output above
```

Installing is not pinning: an explicit `install` request does not change the
project.

### 3. `pin`: record the project's Python

```console
$ pyfaro python pin 3.13
Pinned `.python-version` to `3.13`
$ cat .python-version
3.13
$ pyfaro python pin
3.13
$ pyfaro python pin --rm
Removed Python version file at `.python-version`
```

- `pin <REQUEST>` writes `.python-version`.
- `pin` with no request displays the current pin.
- `pin --rm` removes it.

uv checks a new pin against the project's `requires-python`:

```console
$ pyfaro python pin 3.9
warning: No interpreter found for Python 3.9 in managed installations or search path
error: The requested Python version `3.9` is incompatible with the project `requires-python` value of `>=3.12`.
error: uv python pin failed (exit status: 2); see uv's output above
```

Pins and `requires-python` are what uv reads in later `sync` and `run`
workflows.

### 4. `--offline` and `--uv`

`--offline` disables network access for all three subcommands, `install`
included. `--uv <path>` selects the uv executable. uv 0.9.26 or newer is
required. Requests are passed as separate arguments with no shell. Use `--`
before a request that starts with a dash.

## Project selection

Options go before or after the subcommand.

| Form | Meaning |
| --- | --- |
| `pyfaro python …` | uv works from the current directory, including its ancestors. |
| `pyfaro python --project <PATH> …` | Run uv in that exact directory, which must contain a valid `pyproject.toml`. |

`--project` lets you pin another project's Python from anywhere:

```console
$ cd other
$ pyfaro python pin 3.12 --project ../acme-widget
Pinned `.python-version` to `3.12`
$ pyfaro python --project ../acme-widget pin
3.12
$ pyfaro python pin 3.12 --project ../nope
error: could not resolve project directory ../nope: No such file or directory (os error 2)
```

!!! note
    Without `--project`, `pin` writes `.python-version` in uv's discovered
    location. In a directory with no project, that is the current directory.

## What it writes, and what it never writes

**Writes:** `.python-version` (`pin`) and uv's managed interpreters
(`install`, uv's own cache directory).

**Never writes:** `pyproject.toml`, `uv.lock` or `.venv`. `list` writes
nothing. Pyfaro keeps no installer, catalog or cache of its own.

The everyday commands ([`init`](init.md), [`sync`](sync.md),
[`run`](run.md)) pass `--no-python-downloads`, so they never download an
interpreter behind your back.

## Errors you may hit

| Message | Fix |
| --- | --- |
| `No Python version file found; specify a version to create one` | `pin` with no request and no existing pin. Give a version. |
| `The requested Python version … is incompatible with the project requires-python` | Pin a version inside the range, or update `requires-python`. |
| `No download found for request: …` | Check the version spelling with `pyfaro python list`. |
| `could not resolve project directory …` | `--project` points to a missing directory. |
| `No Python version file found` on `pin --rm` | There is no pin to remove. |

## Exit codes

| Code | Meaning |
| --- | --- |
| `0` | The operation succeeded. |
| `1` | A Pyfaro error, such as a bad `--project`. |
| `2` | A usage error, or uv failed with status 2 (as in all examples above). |
| other | uv's numeric status is forwarded. |
| `130` | Cancelled. |

## See also

- [Python versions workflow](../workflows/python-versions.md)
- [`pyfaro sync`](sync.md): build the environment with the pinned Python.
- [`pyfaro create`](create.md): `--python` sets the minimum version.
