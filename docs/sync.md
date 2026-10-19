# Synchronizing a project

`pyfaro sync [PATH]` delegates to `uv sync` in the selected directory. The
path defaults to the current directory; uv discovers the nearest project
from there. This command is in the unreleased `0.1.0-alpha.2` development
build, not the published `0.1.0-alpha.1` crate.

```sh
pyfaro sync
pyfaro sync ./my-project --locked
pyfaro sync ./my-project --frozen --offline
```

Default mode lets uv update its own `uv.lock` before synchronizing `.venv`.
`--locked` requires a present, up-to-date lock and refuses to modify it when
project metadata changed. `--frozen` requires an existing lock but deliberately
skips the freshness check, so newly declared dependencies may be absent from
the environment. The two modes are mutually exclusive. Pyfaro never creates
an additional lockfile or resolves dependencies itself.

Use `--uv <executable>` to select uv and `--offline` to disable network access.
Pyfaro accepts stable uv versions >=0.9.26; CI tests 0.9.26. An installed
compatible Python interpreter is required because Pyfaro disables uv's Python
downloads. uv owns dependency sources, indexes, resolution, virtualenv creation,
and package installation. It may remove extraneous packages during an exact
sync. uv's stdout and stderr stream directly; Pyfaro returns uv's failure
status, or 130 after cancellation and process-tree cleanup.
