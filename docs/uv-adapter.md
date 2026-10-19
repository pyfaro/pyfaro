# Delegating to uv

Pyfaro selects an explicitly supplied uv executable or finds `uv` on PATH.
It resolves the executable before entering a project directory, checks its
version, and runs it directly without a shell. Stable uv versions at least
`0.9.26` are accepted; `0.9.26` is the version pinned in integration tests.
Newer versions are accepted but not claimed as tested.

The shared adapter passes OS-native arguments and a per-command working
directory to uv. It inherits the caller's standard streams, so output appears
as uv emits it, and returns uv's numeric failure status. Pyfaro reports its
own discovery, version, spawn, and process-cleanup errors separately. It never
installs uv or downloads Python; uv receives `--no-python-downloads`.

`UV_PROJECT`, `UV_WORKING_DIR`, `UV_FROZEN`, `UV_LOCKED`, and `UV_NO_SYNC`
environment overrides are removed before delegation so they cannot redirect
the project or change an explicit locking or syncing mode. Pyfaro leaves the
project's uv indexes, sources, and dependency settings to uv.

Linux keeps the existing process-group and child-subreaper cleanup used by
`init`. macOS uses a process group; Windows uses a Job Object. Cancellation
stops the owned process tree and returns `130`; an unconfirmed cleanup is
reported as an error. The adapter is shared infrastructure for later commands.
`init --apply` remains Linux-only because its file-restoration contract has
only been validated there.
