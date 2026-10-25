# pyfaro build

`pyfaro build` builds your project into a source distribution (`.tar.gz`) and
a wheel (`.whl`), by delegating to `uv build`. Use it to check that the
package builds, and before uploading it. It does not publish anything.

Pyfaro does not choose a build backend. It builds with whatever
`[build-system]` in your `pyproject.toml` declares, so the projects from
`pyfaro create` build with `uv_build`, and a project that uses `hatchling` or
`setuptools` keeps using them.

## Quick example

```console
$ pyfaro build
Building source distribution (uv build backend)...
Building wheel from source distribution (uv build backend)...
Successfully built dist/acme_widget-0.1.0.tar.gz
Successfully built dist/acme_widget-0.1.0-py3-none-any.whl
built dist/acme_widget-0.1.0.tar.gz
built dist/acme_widget-0.1.0-py3-none-any.whl
```

uv's lines go to standard error. The `built` lines go to standard output and
list the artifacts this run created or rewrote, so a script can read them.

## Scenarios

### 1. Build both kinds

With no flag, or with both `--sdist` and `--wheel`, uv builds the source
distribution first and then a wheel from it. This is the build that proves
the sdist is complete.

### 2. Only the sdist, or only the wheel

```console
$ pyfaro build --sdist
built dist/acme_widget-0.1.0.tar.gz
$ pyfaro build --wheel
built dist/acme_widget-0.1.0-py3-none-any.whl
```

Files from earlier runs that were not rewritten are not listed.

### 3. Choose the output directory

`--out-dir` is relative to the directory you run the command from. The default
is `dist` in the project root.

```console
$ pyfaro build --wheel --out-dir artifacts
built artifacts/acme_widget-0.1.0-py3-none-any.whl
$ cd src
$ pyfaro build
built ../dist/acme_widget-0.1.0.tar.gz
built ../dist/acme_widget-0.1.0-py3-none-any.whl
```

### 4. Remove stale artifacts with `--clear`

Old versions stay in `dist` until you remove them. `--clear` empties the
output directory before the build:

```console
$ ls dist
acme_widget-0.1.0-py3-none-any.whl  acme_widget-0.1.0.tar.gz
$ pyfaro build --clear
built dist/acme_widget-0.1.0.tar.gz
built dist/acme_widget-0.1.0-py3-none-any.whl
```

Because `--clear` deletes, Pyfaro refuses it when the output directory is the
project itself or a parent of it:

```console
$ pyfaro build --clear --out-dir .
error: --clear would delete the project: the output directory /work/acme-widget is or contains the project at /work/acme-widget; choose another --out-dir
$ echo $?
2
```

### 5. Other build backends

```console
$ grep -A2 build-system pyproject.toml
[build-system]
requires = ["hatchling"]
build-backend = "hatchling.build"
$ pyfaro build
Successfully built dist/acme_widget-0.1.0.tar.gz
Successfully built dist/acme_widget-0.1.0-py3-none-any.whl
built dist/acme_widget-0.1.0.tar.gz
built dist/acme_widget-0.1.0-py3-none-any.whl
```

The `uv_build` backend works offline. Other backends are downloaded by uv into
a build environment, so they need network access, or a warm cache together
with `--offline`.

### 6. Choose uv with `--uv`, work offline with `--offline`

`--uv <path>` selects the uv executable (default `uv`; 0.9.26 or newer is
required, and 0.9.26 is the tested version). `--offline` disables uv's
network access.

## Project selection

Without options, Pyfaro looks for the nearest `pyproject.toml` in the current
directory and its ancestors. `--project <PATH>` uses exactly that directory,
which must hold a `pyproject.toml`.

## What it writes, and what it never writes

**Writes** (through uv): the artifacts, and a `.gitignore` containing `*` in
the output directory if uv creates that directory.

**Never writes:** `pyproject.toml`, `uv.lock` or `.venv`. Pyfaro does not
change the build backend, and it passes `--no-python-downloads`, so a
compatible Python must already be installed. Use
[`pyfaro python install`](python.md) to get one.

## Errors you may hit

| Message | Fix |
| --- | --- |
| `no pyproject.toml found from …; use --project <PATH>` | `cd` into a project, or pass `--project`. |
| `uv build failed (exit status: N); see uv's output above` | Read uv's error above it, such as `Expected a Python module at: src/acme_widget/__init__.py`. |
| `uv build succeeded but wrote no wheel (.whl) for … in …` | The build backend wrote a different file name. Check `[build-system]` and the project name. |
| `--clear would delete the project` | Choose an output directory below or beside the project. |
| `the network was disabled` | Run without `--offline`, or warm uv's cache first. |
| `could not find uv at …` | Install uv or pass `--uv <path>`. |

## Exit codes

| Code | Meaning |
| --- | --- |
| `0` | The artifacts were built. |
| `1` | A Pyfaro error, or uv failed with status 1. |
| `2` | A usage error, or uv failed with status 2. |
| other | uv's failure status is forwarded after `uv build failed (exit status: N)`. |
| `130` | Cancelled. |

## See also

- [`pyfaro create`](create.md): the projects it generates build with `uv_build`.
- [`pyfaro check`](check.md) and [`pyfaro test`](test.md): verify the code before you build.
- [`pyfaro sync`](sync.md): create the project environment.
