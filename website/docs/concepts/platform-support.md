---
description: "Which operating systems and architectures Pyfaro supports, and what is tested on Linux, macOS and Windows."
---

# Platform support

| Platform | Prebuilt archive | Implemented commands |
| --- | --- | --- |
| Linux x86_64 | yes | all |
| Linux aarch64 | yes | all |
| macOS arm64 | yes | all |
| macOS x86_64 | yes | all |
| Windows x86_64 | yes | all |

Continuous integration runs the implemented workflows on Linux, macOS and
Windows, including `init --apply` with its rollback and cancellation
guarantees. The macOS x86_64 archive is cross-compiled and is not executed
during the release.

!!! note "`init --apply` before 0.6.0"
    Releases up to `0.5.0-beta.1` apply `init` on Linux only; on macOS and
    Windows they preview the plan and stop at `--apply`. From `0.6.0-beta.1`
    every platform applies it. See [`pyfaro init`](../commands/init.md).

## Process handling

Pyfaro runs tools without a shell and cleans up their process tree on
cancellation, which is how `init --apply` can restore your files when you press
Ctrl+C. The mechanism depends on the platform:

- **Linux** uses process groups and a child subreaper.
- **macOS** uses a process group and sends SIGINT before forcing cleanup.
- **Windows** places the child in a Job Object that is closed with the process.

On Linux and macOS, `pyfaro dev` gives the process tree about five seconds to
shut down after Ctrl+C. On Windows, the console delivers Ctrl+C to every
attached process, which Uvicorn's reloader also relies on; `pyfaro dev` waits
about five seconds for the tree to end by itself and keeps running if it does
not, so a reload does not stop it. A second Ctrl+C terminates the job at once.
Every other command terminates the job immediately.

On Windows, Pyfaro refuses to start `.bat` and `.cmd` wrappers, since they
would invoke a shell. Run the underlying executable instead.

The runtime requirements are the same everywhere: [uv](../getting-started/installation.md#install-uv)
0.9.26 or newer and a Python interpreter.

## Next steps

- [Installation](../getting-started/installation.md).
- [Tested versions](../reference/tested-versions.md).
