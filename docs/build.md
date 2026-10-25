# Building distributions

`pyfaro build` delegates to `uv build` and builds a source distribution
(sdist) and a wheel of the selected project. Without `--project`, Pyfaro finds
the nearest `pyproject.toml` in the current directory or its ancestors;
`--project <PATH>` selects the exact directory and never falls back to a
parent. Symlinks are resolved first. Pyfaro always passes the project root to
uv explicitly, so the result does not depend on the working directory.

```sh
pyfaro build
pyfaro build --wheel --out-dir artifacts
pyfaro build --sdist --clear
pyfaro build --project ./my-project --offline
```

## Kinds

With neither `--sdist` nor `--wheel`, or with both, uv builds its default: a
source distribution, then a wheel built from that source distribution.
`--sdist` builds only the source distribution; `--wheel` builds only the
wheel, directly from the project.

## Output directory

Artifacts go to `dist` in the project root. `--out-dir <DIR>` chooses another
directory; a relative path is relative to the **current directory**, not to
the project. `--clear` forwards uv's `--clear`: uv empties the output
directory before building, so stale artifacts disappear. Because that is
destructive, Pyfaro refuses `--clear` (usage error, exit status 2) when the
output directory is the project root or one of its ancestors. Without
`--clear`, existing files stay; artifacts of the same name are overwritten.

## Build backend

Pyfaro never edits `pyproject.toml` and never passes a build backend to uv.
Whatever `[build-system]` declares (`uv_build`, `hatchling`, `setuptools`, or
no table at all, which uv treats as setuptools) is what builds the project,
so PEP 517 and PEP 518 configuration is preserved. uv's own `uv_build` backend
needs no network; other backends are installed by uv into an isolated build
environment, which needs the package index (or a warm uv cache with
`--offline`). Pyfaro passes `--no-python-downloads`, so a compatible Python
must already be installed.

## Output

uv's progress and errors stream through on standard error. When uv succeeds,
Pyfaro lists the artifacts this run created or rewrote in the output
directory, one per line on standard output, with the path relative to the
current directory when it is inside or near it:

```text
built dist/acme_widget-0.1.0.tar.gz
built dist/acme_widget-0.1.0-py3-none-any.whl
```

Files that were already there and not rewritten are not listed. Pyfaro
detects them by file name, modification time and size before and after the
run, and matches names with PEP 625 and the wheel file name rules (the
normalized distribution name, so `Acme.Widget` produces `acme_widget-…`).
It does not open the archives.

If uv succeeds but an expected kind is missing (a source distribution when
building both kinds or `--sdist`, a wheel when building both kinds or
`--wheel`), Pyfaro fails with exit status 1 and says which kind is missing
and where it looked, which points to a backend that wrote a different name.

## Exit status

| Code | Meaning |
| --- | --- |
| `0` | The artifacts were built. |
| `1` | A Pyfaro error: no project, missing or unsupported uv, or an expected artifact missing. |
| `2` | A usage error (such as an unsafe `--clear`), or uv failed with status 2. |
| other | uv's failure status, forwarded after `uv build failed (exit status: N)`. |
| `130` | Cancelled. Child processes are cleaned up. |

There are no workspace options (`--package`, `--all-packages`) and no
passthrough arguments to uv yet.
