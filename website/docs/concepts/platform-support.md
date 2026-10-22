# Platform support

| Platform | Prebuilt archive | Implemented commands | `init --apply` |
| --- | --- | --- | --- |
| Linux x86_64 | yes | all | yes |
| Linux aarch64 | yes | all | yes |
| macOS arm64 | yes | all except `init --apply` | no |
| macOS x86_64 | yes | all except `init --apply` | no |
| Windows x86_64 | yes | all except `init --apply` | no |

Continuous integration runs the implemented workflows on Linux, macOS and
Windows. The macOS x86_64 archive is cross-compiled and is not executed during
the release.

## `init --apply` is Linux-only

!!! warning "Linux only for now"
    `pyfaro init` without flags prints a plan on every platform. Applying it
    with `--apply` works on **Linux only** in this alpha series: `init` rewrites
    `pyproject.toml` and `uv.lock`, and restores them if uv fails or you cancel.
    That restoration guarantee has only been validated on Linux.

On macOS and Windows, run `pyfaro init` to read the plan and make the changes
yourself, or run `pyfaro init --apply` on Linux, for example in a container.
See [`pyfaro init`](../commands/init.md).

## Process handling

Pyfaro runs tools without a shell and cleans up their process tree on
cancellation. The mechanism depends on the platform:

- **Linux** uses process groups and a child subreaper.
- **macOS** uses a process group and sends SIGINT before forcing cleanup.
- **Windows** places the child in a Job Object that is closed with the process.

On Windows, Pyfaro refuses to start `.bat` and `.cmd` wrappers, since they
would invoke a shell. Run the underlying executable instead.

The runtime requirements are the same everywhere: [uv](../getting-started/installation.md#install-uv)
0.9.26 or newer and a Python interpreter.

## Next steps

- [Installation](../getting-started/installation.md).
- [Tested versions](../reference/tested-versions.md).
