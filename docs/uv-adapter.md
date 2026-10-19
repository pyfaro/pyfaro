# Delegating to uv

Pyfaro selects an explicitly supplied uv executable or finds `uv` on PATH.
It resolves the executable before entering a project directory, checks its
version, and runs it directly without a shell. Stable uv versions at least
`0.9.26` are accepted; `0.9.26` is the version pinned in integration tests.
Newer versions are accepted but not claimed as tested.

The shared adapter passes OS-native arguments and a per-command working
directory to uv. Delegated operations inherit stdout and stderr for streamed
output. They use null stdin until interactive execution is implemented for
`run`; the version probe also uses null stdin and a temporary stdout file.
Pyfaro returns uv's numeric failure status and reports its
own discovery, version, spawn, and process-cleanup errors separately. It never
installs uv or downloads Python; uv receives `--no-python-downloads`.

`UV_PROJECT`, `UV_WORKING_DIR`, `UV_FROZEN`, `UV_LOCKED`, and `UV_NO_SYNC`
environment overrides are removed before delegation so they cannot redirect
the project or change an explicit locking or syncing mode. Pyfaro leaves the
project's uv indexes, sources, and dependency settings to uv.

Linux keeps the existing process-group and child-subreaper cleanup used by
`init`. macOS uses a process group and sends SIGINT before forcing cleanup.
Windows assigns a suspended child to a kill-on-close Job Object before it
runs, stops remaining job members after the leader exits, and confirms the
job's active-process list is empty.
Windows batch wrappers are rejected because Rust would invoke a shell for
them. Cancellation stops the owned process tree and returns `130`; an unconfirmed cleanup is
reported as an error. The adapter is shared infrastructure for later commands.
`init --apply` remains Linux-only because its file-restoration contract has
only been validated there.
