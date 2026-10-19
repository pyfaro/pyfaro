# Managing dependencies

`pyfaro add` and `pyfaro remove` delegate dependency edits and resolution to
uv. Run them in a project directory or a descendant. Pyfaro finds the nearest
`pyproject.toml`; `--project <PATH>` selects an exact directory and requires
its own `pyproject.toml`.

```sh
pyfaro add "requests>=2,<3"
pyfaro add --dev "pytest>=8"
pyfaro add --group quality "ruff==0.14.0"
pyfaro remove requests
pyfaro remove --dev pytest
pyfaro remove --group quality ruff
pyfaro add --project ./app --offline "local-package>=1"
```

`add` accepts uv's PEP 508 requirement specifiers; `remove` accepts dependency
names. Multiple arguments are allowed. Use `--` before a requirement that
starts with a dash. `--dev` and `--group <NAME>` are mutually exclusive; omit
both for `[project].dependencies`. uv writes the corresponding dependency group
and owns changes to `pyproject.toml`, `tool.uv.sources`, and `uv.lock`. Pyfaro
passes `--no-sync`, so these commands update the lock without installing into
`.venv`; run `pyfaro sync` when you want the environment updated.

`--offline` disables uv network access, and `--uv <executable>` selects a uv
binary. Pyfaro accepts stable uv versions >=0.9.26; integration tests use the
pinned 0.9.26. uv's solver output streams to the terminal, including reasons
for conflicts or packages unavailable offline. Pyfaro returns uv's numeric
failure code. It invokes uv directly with OS-native arguments, never through
a shell, and cancellation returns 130 after process cleanup.
