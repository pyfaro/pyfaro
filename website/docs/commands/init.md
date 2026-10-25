# pyfaro init

`pyfaro init` prepares a standalone project for uv-backed development: it adds
the `[tool.pyfaro]` opt-in to `pyproject.toml` and creates or refreshes
`uv.lock`. Use it to adopt an existing project, or to bootstrap a bare one.
By default it only **previews** the plan. Nothing is written until you pass
`--apply`.

!!! warning "Linux only for now"
    `init --apply` works on Linux only in the current alpha series. On other
    platforms the preview still works, but `--apply` stops with an explicit
    unsupported-platform error before running uv or changing any file. Use
    [`pyfaro create`](create.md) there, which works everywhere.

## Quick example

```console
$ pyfaro init demo
Initialization plan for demo:
  Run uv init --bare (no Git, README, or Python pin files).
  Ensure the following configuration in pyproject.toml:

[tool.pyfaro]
schema-version = 1

  Run uv lock --no-build to create or update uv.lock.
Preview only: rerun with --apply to initialize this project.
$ pyfaro init demo --apply
```

## Scenarios

### 1. Preview a new project

The path defaults to the current directory. In a directory without a
`pyproject.toml`, the plan is to run `uv init --bare`, add the Pyfaro table
and lock. The preview creates no file or directory and does not even need uv
installed.

### 2. Apply it

```console
$ pyfaro init demo --apply
Initialization plan for demo:
  …
Initialized project `demo`
Using CPython 3.13.11
Resolved 1 package in 1ms
Initialized demo with Pyfaro and uv.
$ cat demo/pyproject.toml
[project]
name = "demo"
version = "0.1.0"
requires-python = ">=3.13"
dependencies = []

[tool.pyfaro]
schema-version = 1
```

The result is just `pyproject.toml` and `uv.lock`. No Git repository, README,
`.python-version` or application source is created. Commit
`pyproject.toml` and `uv.lock` together.

!!! note
    `init` does not install packages or create `.venv`. Run
    [`pyfaro sync`](sync.md) afterwards.

### 3. Adopt an existing project

An existing project needs a standard `[project]` table. `init` keeps
everything it finds (dependencies, `[tool.uv]`, `[tool.ruff]`, comments, build
settings) and only adds what is missing:

```console
$ pyfaro init old --apply --offline
…
$ cat old/pyproject.toml
[project]
name = "old"
version = "1.2.3"
requires-python = ">=3.11"
dependencies = []

# keep me
[tool.ruff]
line-length = 100

[tool.pyfaro]
schema-version = 1
```

If `[tool]` or its `pyfaro` entry is written as an inline table, the preview
prints the complete proposed `pyproject.toml` so you can see the exact form:

```console
$ pyfaro init inl3
Initialization plan for inl3:
  Proposed pyproject.toml:
tool = {ruff = {line-length = 100}, pyfaro = { schema-version = 1 } }
[project]
name="i"
version="0.1.0"

  Run uv lock --no-build to create or update uv.lock.
Preview only: rerun with --apply to initialize this project.
```

### 4. Run it again (idempotence)

Applying to an already initialized project is safe. The plan only lists what
is left to do:

```console
$ pyfaro init demo --apply
Initialization plan for demo:
  [tool.pyfaro] is already configured.
  Run uv lock --no-build to create or update uv.lock.
Using CPython 3.13.11
Resolved 1 package in 0.51ms
Initialized demo with Pyfaro and uv.
```

uv re-checks the lock without upgrading anything. If dependency metadata or an
index changed meanwhile, uv may still update a stale lock.

### 5. Work offline with `--offline`

`--offline` disables uv's network access. A project without dependencies, or
whose metadata is already cached, locks fine offline:

```console
$ pyfaro init old --apply --offline
```

### 6. Choose uv with `--uv`

`--uv <path>` selects the uv executable (a path, or a name looked up on
`PATH`). The default is `uv`. A relative path is resolved once, before
entering the project directory.

```console
$ pyfaro init demo --apply --uv /nonexistent/uv
error: could not find uv at /nonexistent/uv; fix --uv <path> or install uv (https://docs.astral.sh/uv/getting-started/installation/, or `curl -LsSf https://astral.sh/uv/install.sh | sh`)
```

uv 0.9.26 or newer (stable) is required. Pyfaro never installs uv or Python,
and it passes `--no-python-downloads`, so a compatible interpreter must
already be installed.

## Project selection

`init` takes a **positional path** and uses that exact directory. It does
**not** search ancestor directories like [`sync`](sync.md) or
[`run`](run.md) do, so creating a project stays deliberate. There is no
`--project` option. Relative paths and paths with spaces work, and symlinks
in the path are resolved.

## What it writes, and what it never writes

**Writes** (with `--apply` only): `pyproject.toml` (new, or with the missing
`[tool.pyfaro]` table added) and `uv.lock`.

**Never writes:** a virtual environment, installed packages, dependency
upgrades, a Git repository, README, `.python-version` or source files. Your
dependencies, `[tool.uv]` sources and indexes, Ruff, ty and pytest settings
and the build system are preserved.

If uv fails or you press Ctrl+C, Pyfaro restores the previous bytes of
`pyproject.toml` and `uv.lock` (removing incomplete new ones). Do not run
another tool that writes those files at the same time.

## Errors you may hit

| Message | Fix |
| --- | --- |
| `expected a standard [project] table in pyproject.toml` | The project uses Poetry-only metadata. Migrate it to `[project]` first. |
| `tool.pyfaro.schema-version must be the integer 1` | Fix or remove the unsupported `[tool.pyfaro]` value. Unknown keys are rejected too. |
| `could not find uv …` | Install uv (>= 0.9.26) as the message says, or pass `--uv <path>`. |
| uv's own error, then `uv … failed` | Read uv's output above. Dynamic or source dependencies that need a build can fail with `--no-build`. |
| unsupported platform on `--apply` | Run it on Linux for now. |

uv workspaces, and projects below a workspace root, are rejected before
anything is modified. Symlinked `pyproject.toml` or `uv.lock` files are
rejected too.

## Exit codes

| Code | Meaning |
| --- | --- |
| `0` | The preview was shown, or the project was initialized. |
| `1` | A Pyfaro configuration or filesystem error, including a missing uv. |
| `2` | A usage error, or a failing uv exit code of `2` (forwarded as is). |
| other | uv's numeric failure status is forwarded. |
| `130` | Cancelled. The process tree was stopped first. |

## See also

- [`pyfaro sync`](sync.md): create the environment from the new lock.
- [`pyfaro create`](create.md): generate a brand-new project instead.
- [`pyfaro doctor`](doctor.md): check the setup.
