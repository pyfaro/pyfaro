# Experimental project initialization

`pyfaro init` is the first vertical workflow in `0.1.0-alpha.1`. It prepares
a standalone project for uv-backed development without installing packages.
It is experimental; the configuration and CLI are not stable contracts yet.

## Requirements

- **Linux for `--apply` in this alpha series.** macOS/Windows support builds,
  help/version, and non-writing previews, but applying returns an explicit
  unsupported-platform error before invoking uv or modifying project files.
  Broader platform support remains on the existing roadmap.
- uv installed separately, selected through `--uv <executable>` or PATH.
- uv 0.9.26 is the pinned, tested version. Stable versions >=0.9.26 are
  accepted, but newer versions are not claimed as tested.
- An existing Python interpreter compatible with the project. Pyfaro
  passes `--no-python-downloads` to uv; it does not install uv or Python.
- An existing project must have standard `[project]` metadata in
  `pyproject.toml`. Poetry-only metadata requires migration first.

## Preview, then apply

```sh
pyfaro init ./my-project
pyfaro init ./my-project --apply
```

The path defaults to the current directory. Unlike `sync` and `run`, `init`
uses the exact directory to initialize, not an ancestor-project search;
this keeps creation of a new project deliberate. Relative paths and paths with
spaces are supported; existing symlinks in the directory path are resolved.

Without `--apply`, Pyfaro validates the existing configuration and displays
the proposed metadata changes and the uv operations. It creates no files
or directories and does not require or invoke uv. Dependency resolution
is not simulated, so the preview cannot promise that `uv lock` will succeed.
When `[tool]` or its `pyfaro` entry is an inline table, the preview displays
the full proposed `pyproject.toml` so the inline entry appears in its actual
form.

With `--apply`, Pyfaro:

1. Checks uv availability and version before modifying project files.
2. For a new project, runs uv's bare initialization, without creating Git,
   README, Python pin, application source, or workspace-membership files.
3. Adds the following opt-in settings, preserving unrelated TOML content
   and comments. Existing compatible settings are retained:

   ```toml
   [tool.pyfaro]
   schema-version = 1
   ```

4. Runs `uv lock --no-build` in the project directory. uv owns resolution
   and the standard `uv.lock` file. No `uv sync`, dependency upgrades,
   source builds, or environment installation is requested.

Dependencies, `[tool.uv]` sources/index settings, `[tool.ruff]`, `[tool.ty]`,
pytest settings, build-system configuration, and project files are preserved.
Only missing Pyfaro settings are added. Unknown Pyfaro keys or a schema
version other than the integer `1` are rejected before writing.

Reapplying to an unchanged project is idempotent. uv rechecks its lockfile
without `--upgrade`; external metadata or index changes may still require
uv to update a stale lockfile. Commit `pyproject.toml` and `uv.lock` together.

After initialization, published `alpha.2` can synchronize the environment
with `pyfaro sync --locked` and execute through it with
`pyfaro run --locked -- python --version`. Users remaining on `alpha.1` must
use uv directly for these workflows:

```sh
cd my-project
uv sync --locked
uv run --locked python --version
```

These explicit uv commands can install project dependencies and run Python;
they are not performed by `pyfaro init`.

## Options and failures

- `--offline`: disable uv network access. Dependency resolution needs
  available local metadata/cache; a dependency-free project works offline.
- `--uv <path>`: select an executable without invoking a shell. A relative
  executable path or PATH lookup is resolved once before entering the project
  directory; the same validated binary is used for every uv operation.
- The `UV_PROJECT`, `UV_WORKING_DIR`, `UV_FROZEN`, and
  `UV_INIT_BUILD_BACKEND` environment overrides are ignored for initialization
  so they cannot redirect the operation, skip locking, or create unplanned
  build-backend files. Project dependency/source/index settings remain uv-owned.
- uv stdout/stderr are streamed. A numeric uv failure status is forwarded;
  Pyfaro's own configuration/filesystem errors return `1`. CLI usage errors
   return `2`. Cancellation returns `130` after stopping the owned Linux
   process group, reaping the leader and adopted descendants, and confirming
   group exit.

If a file/uv operation fails or is cancelled, Pyfaro restores the previous bytes
of `pyproject.toml` and `uv.lock`, removing incomplete new versions of those
files. Existing file permissions are retained. If restoration itself fails,
the error identifies that recovery is incomplete; inspect both files before
retrying. Newly created empty directories and uv's external cache may remain.
If process cleanup cannot be confirmed within its deadline, Pyfaro reports
that failure and skips automatic file restoration rather than racing a
potentially live writer. Linux child-subreaper support is required; failure
to enable it is reported before project-file mutation.
This is error recovery, not a crash-proof multi-file transaction. Do not run
another metadata/lockfile writer concurrently with initialization.

## Alpha limits

- uv workspaces and projects beneath workspace roots are rejected before
  modification. Workspace support remains on the existing roadmap.
- Metadata/lockfile symlinks are rejected to avoid replacing their targets.
- Dynamic/source dependencies that require a build to determine metadata
  may fail with `--no-build`; the previous project files are then restored.
- There is no preset selection or automatic quality-tool dependency setup.
  `sync` and `run` require `alpha.2` or later.

## Verification

The test seams are the public CLI and resulting project files, the external
uv process boundary, and the configuration transformation. Successful
integration cases use real uv 0.9.26 offline, including a local dependency.
They cover new/existing projects, preservation, idempotence, and non-ASCII
paths with spaces. Portable Rust process fixtures cover uv failures and
process-tree cancellation and confirm that descendant PIDs have been reaped.
Linux additionally tests actual SIGINT delivery through the CLI. CI runs the
full `init --apply` workflow suite on Linux and checks compilation, previews,
and the unsupported-apply contract on macOS and Windows. Real `sync` and
`run` smoke tests execute on all three platforms.
