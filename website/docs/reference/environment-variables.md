# Environment variables

The `pyfaro` executable has no variables of its own: it reads a few standard
ones to find environments and uv configuration, and removes others before
calling uv so that they cannot change what a command means. Only the
[install scripts](#read-by-the-install-scripts) have `PYFARO_*` variables. This
page lists every variable you can set.

## Read by Pyfaro

| Variable | Used by | Effect |
| --- | --- | --- |
| `UV_PROJECT_ENVIRONMENT` | all commands that look up tools | Project environment instead of `.venv`. A relative path is resolved from the project root. See [Environments and tools](../concepts/environments-and-tools.md). |
| `VIRTUAL_ENV` | tool lookup, `doctor`, `tools` | The activated environment, searched after the project environment and before `PATH`. An empty value is ignored. |
| `PATH` | tool lookup | Last place searched for Ruff, ty, pytest and Python, and where `uv` is found unless `--uv` is given. |
| `UV_CONFIG_FILE` | `doctor` | Makes `doctor` read this file as uv's configuration, relative to the project root. |
| `UV_NO_CONFIG` | `doctor` | Turns off uv configuration discovery in `doctor`'s checks. |
| `UV_NO_GROUP`, `UV_NO_DEV`, `UV_NO_DEFAULT_GROUPS` | `doctor` | Taken into account when `doctor` decides which dependency groups are selected. |
| `XDG_CONFIG_HOME`, `HOME` | `doctor` (Linux, macOS) | Locate the user-level `uv/uv.toml`. |
| `XDG_CONFIG_DIRS` | `doctor` (Linux, macOS) | Locate system-level `uv/uv.toml` files (default `/etc/xdg`, and `/etc/uv/uv.toml`). |
| `APPDATA`, `PROGRAMDATA` | `doctor` (Windows) | Locate the user-level and system-level `uv\uv.toml`. |
| `NO_COLOR`, `CLICOLOR_FORCE` | help and usage errors | Honored by the argument parser: `NO_COLOR` turns the colors of `--help` and usage errors off, `CLICOLOR_FORCE=1` turns them on when output is not a terminal. Pyfaro adds no color to its other messages, and the tools' own output is not touched. |

Boolean uv variables (`UV_NO_DEV` and friends) are true for `1`, `true`, `t`,
`yes`, `y` and `on`, case-insensitive.

## Read by the install scripts

These belong to [`install.sh` and `install.ps1`](../getting-started/installation.md#install-with-the-script),
not to the `pyfaro` executable.

| Variable | Effect |
| --- | --- |
| `PYFARO_VERSION` | Version to install instead of the newest release (also pins or downgrades). |
| `PYFARO_INSTALL_DIR` | Directory for the binary. Default `~/.local/bin` (Windows: `%USERPROFILE%\.local\bin`). |
| `PYFARO_DOWNLOAD_BASE` | Release download URL prefix, for testing mirrors only. `https://` or `file://`. |
| `GITHUB_TOKEN` | Optional; sent to `api.github.com` only, to avoid rate limits when finding the newest release. |

## Cleared before calling uv

These variables are removed from the environment of every uv process that
Pyfaro starts, so that they cannot redirect the project or change a locking
or syncing mode you chose on the command line.

| Variable | Why |
| --- | --- |
| `UV_PROJECT`, `UV_WORKING_DIR` | Pyfaro selects the project directory itself. |
| `UV_FROZEN`, `UV_LOCKED` | Locking mode comes from `--frozen` and `--locked`. |
| `UV_NO_SYNC` | Would change the syncing mode that Pyfaro chose. |
| `UV_ISOLATED` | Would isolate the run environment. |
| `UV_INIT_BUILD_BACKEND` | Would change the project that `init` creates. |

In addition:

- `add` and `remove` clear `UV_DEV`, so the dependency group follows `--dev`
  and `--group` only.
- The `python` commands clear `UV_NO_PROJECT`, `UV_NO_CONFIG` and `UV_PYTHON`,
  so that uv honors project metadata and `.python-version`.

Everything else, including `UV_INDEX_URL`, `UV_CACHE_DIR`, `UV_LINK_MODE` and
proxy settings, passes through to uv unchanged.

## Set by Pyfaro

Pyfaro does not export variables of its own. The one exception is the `env`
table of a [task](configuration.md#toolpyfarotasks): its variables are added
to the environment of that task's process (the task's values win; `-v` prints
their names, never their values).

On the uv command line, Pyfaro passes `--no-progress`, adds
`--no-python-downloads` except for `python install` and `python list`, and
`--offline` when you pass `--offline`.

## Next steps

- [Environments and tools](../concepts/environments-and-tools.md).
- [How it works](../concepts/how-it-works.md).
