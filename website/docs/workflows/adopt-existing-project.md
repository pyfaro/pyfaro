---
description: "Adopt Pyfaro in a project you already have: preview with pyfaro init, keep your pyproject.toml and uv.lock, and add the quality gate."
---

# Adopt an existing project

Pyfaro works with the `pyproject.toml` and `uv.lock` you already have. This
recipe adopts a small existing project: preview and apply `init`, sync,
diagnose with `doctor`, then add the quality tools.

The example project is a `src/` layout package with a `requests` dependency
and no Pyfaro configuration:

```toml
[project]
name = "legacy"
version = "0.2.0"
requires-python = ">=3.11"
dependencies = ["requests>=2"]

[build-system]
requires = ["uv_build>=0.9.26,<0.10.0"]
build-backend = "uv_build"
```

## 1. Preview, then apply `init`

`init` previews by default and writes nothing:

```console
$ pyfaro init
Initialization plan for .:
  Ensure the following configuration in pyproject.toml:

[tool.pyfaro]
schema-version = 1

  Run uv lock --no-build to create or update uv.lock.
Preview only: rerun with --apply to initialize this project.
```

When the plan looks right, apply it:

```console
$ pyfaro init --apply
Initialization plan for .:
  Ensure the following configuration in pyproject.toml:

[tool.pyfaro]
schema-version = 1

  Run uv lock --no-build to create or update uv.lock.
Using CPython 3.13.11
Resolved 6 packages in 289ms
Initialized . with Pyfaro and uv.
```

Your existing metadata, dependencies and other tools' settings are left
alone; Pyfaro adds only `[tool.pyfaro]` and creates `uv.lock`.

!!! warning "`init --apply` is Linux-only for now"
    On macOS and Windows, `init` previews but refuses to apply. You can add
    `[tool.pyfaro]` with `schema-version = 1` to `pyproject.toml` by hand and
    run `pyfaro sync`; every other command works on all three platforms.

## 2. Sync

```console
$ pyfaro sync
Using CPython 3.13.11
Creating virtual environment at: .venv
Resolved 6 packages in 0.70ms
   Building legacy @ file://~/projects/legacy
      Built legacy @ file://~/projects/legacy
Prepared 1 package in 3ms
Installed 6 packages in 11ms
 + certifi==2026.10.19
 + charset-normalizer==3.5.2
 + idna==3.20
 + legacy==0.2.0 (from file://~/projects/legacy)
 + requests==2.34.2
 + urllib3==2.8.0
```

## 3. Diagnose with `doctor`

[`doctor`](../commands/doctor.md) reports what is configured and what is
missing, without changing anything. Run before the sync, it flags the missing
environment:

```console
$ pyfaro doctor
OK: project: ~/projects/legacy
OK: pyproject.toml: valid Pyfaro configuration
OK: Python: /usr/bin/python3.14 (Python 3.14.8)
OK: uv: /home/you/.local/bin/uv (uv 0.9.26)
WARN: ruff: ruff: not found; install ruff in the project environment (.venv by default), activate an environment, or add it to PATH
WARN: ty: ty: not found; install ty in the project environment (.venv by default), activate an environment, or add it to PATH
WARN: pytest: pytest: not found; install pytest in the project environment (.venv by default), activate an environment, or add it to PATH
ERROR: dependencies: project interpreter ~/projects/legacy/.venv/bin/python is missing; run pyfaro sync
error: doctor found 1 blocking issue(s); follow the actions above and run pyfaro doctor again
```

After `sync` the blocking error is gone. The three warnings remain, because
this project does not have the quality tools yet:

```console
$ pyfaro doctor
OK: project: ~/projects/legacy
OK: pyproject.toml: valid Pyfaro configuration
OK: Python: ~/.local/share/uv/python/cpython-3.13.11-linux-x86_64-gnu/bin/python3.13 (Python 3.13.11)
OK: uv: /home/you/.local/bin/uv (uv 0.9.26)
WARN: ruff: ruff: not found; install ruff in the project environment (.venv by default), activate an environment, or add it to PATH
WARN: ty: ty: not found; install ty in the project environment (.venv by default), activate an environment, or add it to PATH
WARN: pytest: pytest: not found; install pytest in the project environment (.venv by default), activate an environment, or add it to PATH
OK: dependencies: 1 runtime distribution(s) installed in .venv
OK: doctor: no blocking issues
```

Warnings do not fail `doctor`; only `ERROR` lines do (exit code 1).

## 4. Add the quality tools

Put Ruff, ty and pytest in the `dev` group with `add --dev`, then sync. Pin
them to the [tested versions](../reference/tested-versions.md) if you want
results that match these examples; unpinned, uv resolves the latest release.

```console
$ pyfaro add --dev "pytest==9.1.1" "ruff==0.16.10" "ty==0.0.85"
Resolved 14 packages in 11ms
$ pyfaro sync
Resolved 14 packages in 0.70ms
   Building legacy @ file://~/projects/legacy
      Built legacy @ file://~/projects/legacy
Prepared 1 package in 5ms
Uninstalled 1 package in 0.53ms
Installed 8 packages in 22ms
 + iniconfig==2.3.1
 ~ legacy==0.2.0 (from file://~/projects/legacy)
 + packaging==26.3
 + pluggy==1.7.0
 + pygments==2.21.0
 + pytest==9.1.1
 + ruff==0.16.10
 + ty==0.0.85
$ pyfaro tools
uv: /home/you/.local/bin/uv (uv 0.9.26)
ruff: ~/projects/legacy/.venv/bin/ruff (ruff 0.16.10)
ty: ~/projects/legacy/.venv/bin/ty (ty 0.0.85)
pytest: ~/projects/legacy/.venv/bin/pytest (pytest 9.1.1)
```

## 5. Run the first check

Adopted code rarely passes on the first try. Here the module has an unsorted
import block and missing blank lines:

```console
$ pyfaro check
unformatted: File would be reformatted
 --> src/legacy/__init__.py:3:1
  |
2 |
3 +
4 | def status(url):
  |

1 file would be reformatted
I001 [*] Import block is un-sorted or un-formatted
 --> src/legacy/__init__.py:1:1
  |
1 | import requests
  | ^^^^^^^^^^^^^^^
2 |
3 | def status(url):
  |
help: Organize imports
  |
2 |
3 +
4 | def status(url):
  |

Found 1 error.
[*] 1 fixable with the `--fix` option.
All checks passed!

== pyfaro check summary ==
format: failed (exit 1)
lint: failed (exit 1)
type: ok
check: failed (format, lint)
```

Both findings are safe to fix automatically:

```console
$ pyfaro check --fix
fix: format and lint can be fixed automatically; type has no automatic fixes
Found 1 error (1 fixed, 0 remaining).
fix lint: ok
1 file left unchanged
fix format: ok
fix: rechecking without writing
1 file already formatted
All checks passed!
All checks passed!

== pyfaro check summary ==
format: ok
lint: ok
type: ok
check: ok
```

Review the diff with `git diff`, commit, and you are on the same footing as a
project created by Pyfaro.

## Next steps

- Keep it clean: [The quality gate](quality-gate.md).
- Run it in CI: [Continuous integration](continuous-integration.md).
- If something is off, see [Troubleshooting](troubleshooting.md).
- Details: [`init`](../commands/init.md), [`doctor`](../commands/doctor.md),
  [`add` and `remove`](../commands/add-remove.md).
