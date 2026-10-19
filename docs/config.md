# Project configuration

Pyfaro reads `pyproject.toml` from the selected project root. No separate
Pyfaro metadata file is required. A project must have a standard `[project]`
table. The selected root is resolved to a physical path before configuration
is loaded, so nested invocations and symlinks use the same metadata. See the
[project selection rules](sync.md) for discovery and exact `--project` paths.

`[tool.pyfaro]` is optional for `sync` and `run`. Omitting it uses schema
version `1` without writing to the file. `init` explicitly adds this table:

```toml
[tool.pyfaro]
schema-version = 1
```

The only supported setting in this alpha is `schema-version`, and it must be
the integer `1`. Unknown Pyfaro keys, other schema versions, malformed TOML,
and non-table `[project]`, `[tool]`, or `[tool.pyfaro]` values fail before uv
is launched. Errors identify the metadata path and invalid setting. Pyfaro
does not interpret or rewrite `[tool.ruff]`, `[tool.ty]`, `[tool.pytest]`,
`[tool.uv]`, or other tools' sections. `init` preserves comments and unrelated
TOML formatting when it adds its own settings.

The metadata path is resolved from the selected project root, not from the
location where a delegated command happens to run. Pyfaro adds no path-valued
settings in schema version `1`; uv and the individual tools retain ownership
of their own path semantics.
