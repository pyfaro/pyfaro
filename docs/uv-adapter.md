# Delegating to uv

Pyfaro selects an explicitly supplied uv executable or finds `uv` on PATH.
It resolves the executable before entering a project directory, checks its
version, and runs it directly without a shell. Stable uv versions at least
`0.9.26` are accepted; `0.9.26` is the version pinned in integration tests.
Newer versions are accepted but not claimed as tested.

The shared adapter passes OS-native arguments and a per-command working
directory to uv. Delegated operations inherit stdout and stderr for streamed
output. `init` and `sync` use null stdin; `run` passes redirected input
through and relays foreground-terminal input on Unix, where the delegated
process group cannot read directly from the parent's foreground terminal.
Terminal-specific child features that require stdin itself to be a TTY are
not emulated. The version probe uses null stdin and temporary stdout/stderr
files. It reads the first version line and includes up to 4096 bytes of
stderr on failure.
Pyfaro returns uv's numeric failure status and reports its
own discovery, version, spawn, and process-cleanup errors separately. It never
installs uv or downloads Python; uv receives `--no-python-downloads`.

`UV_PROJECT`, `UV_WORKING_DIR`, `UV_FROZEN`, `UV_LOCKED`, `UV_NO_SYNC`, and
`UV_ISOLATED` environment overrides are removed before delegation so they
cannot redirect the project, isolate the run environment, or change an
explicit locking or syncing mode. Pyfaro leaves the project's uv indexes,
sources, and dependency settings to uv.

Linux keeps the existing process-group and child-subreaper cleanup used by
`init`. macOS uses a process group and sends SIGINT before forcing cleanup.
Windows assigns a suspended child to a kill-on-close Job Object before it
runs while preserving the caller's console. It terminates the job after the
leader exits and confirms the active process count reaches zero. macOS checks
process states when Darwin reports
`EPERM` for a group that may contain only zombies.
Windows batch wrappers are rejected because Rust would invoke a shell for
them. Cancellation stops the owned process tree and returns `130`; an unconfirmed cleanup is
reported as an error. The adapter is shared infrastructure for later commands.
`init --apply` remains Linux-only because its file-restoration contract has
only been validated there.
