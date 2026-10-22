# Configuration

Pyfaro reads `pyproject.toml` from the selected [project](projects.md) root.
There is no separate Pyfaro file.

## `[tool.pyfaro]`

The table is optional. Without it, Pyfaro behaves as if it contained the
default values, and never writes to your file.

```toml
[tool.pyfaro]
schema-version = 1
```

`pyfaro create` writes this table, and `pyfaro init --apply` adds it to an
existing project.

| Key | Type | Meaning |
| --- | --- | --- |
| `schema-version` | integer | Version of the configuration format. Must be `1`. |

`schema-version` is the only setting in the alpha series. It exists so that
future versions can change the format without breaking existing projects.

## Validation

Pyfaro validates the file before it launches any tool. These cases fail with
exit code 1 and an error that names the file and the setting:

- malformed TOML;
- a missing `[project]` table, or one that is not a table;
- `[tool]` or `[tool.pyfaro]` that is not a table;
- an unknown key under `[tool.pyfaro]`;
- a `schema-version` other than `1`.

`pyfaro doctor` runs the same validation and reports the result.

## Other tools' settings

Pyfaro does not read, interpret or rewrite `[tool.ruff]`, `[tool.ty]`,
`[tool.pytest]`, `[tool.uv]` or any other section. Each tool owns its own
configuration, so existing settings keep working. When `pyfaro init --apply`
adds its table, it preserves comments and unrelated formatting.

!!! note "Conflicting configuration files"
    `doctor` warns when a tool has settings in more than one place, for
    example `pytest.ini` next to `[tool.pytest.ini_options]`, since the tool
    will only use one of them.

## Next steps

- [`pyfaro init`](../commands/init.md) to adopt an existing project.
- [Adopt an existing project](../workflows/adopt-existing-project.md).
- [Projects](projects.md).
