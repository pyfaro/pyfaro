# Troubleshooting

Start with [`pyfaro doctor`](../commands/doctor.md): it inspects the project
configuration, interpreter, uv and tools without changing anything, and each
`ERROR` or `WARN` line says what to do. This page lists the messages you are
most likely to meet and how to fix them. All messages below are real output.

## Exit codes

| Code | Meaning |
| --- | --- |
| `0` | Success. For `doctor`, warnings alone still exit 0. |
| `1` | Pyfaro's own failure (no project, bad configuration, missing tool, blocking `doctor` finding), or a failing `check`. Commands that run a tool, such as `lint`, `fmt` and `type`, return that tool's status, which is `1` for findings. |
| `2` | Usage error (unknown flag or subcommand), and uv's own exit code `2` is passed through by the delegated commands. |
| `130` | Cancelled with Ctrl-C. |

`sync`, `add`, `remove` and `python` stream uv's output and preserve uv's exit
status. `test` returns pytest's status unchanged: `0` passed, `1` failures,
`5` nothing collected (for example `-k nomatch`). See
[Exit codes](../concepts/exit-codes.md) for the full table.

## No project found

```console
$ pyfaro sync
error: no pyproject.toml found from ~/projects; use --project <PATH>
```

You are outside a project. Pyfaro walks up from the current directory to the
nearest `pyproject.toml`. `cd` into the project, or select it with
`--project <PATH>`.

## The environment does not exist

```console
$ pyfaro test
error: pytest: project environment interpreter not found at ~/projects/demo/.venv/bin/python; run `pyfaro sync` first
```

```console
$ pyfaro doctor
...
ERROR: dependencies: project interpreter ~/projects/demo/.venv/bin/python is missing; run pyfaro sync
error: doctor found 1 blocking issue(s); follow the actions above and run pyfaro doctor again
```

Nothing in Pyfaro syncs implicitly. Run `pyfaro sync`, and again after
changing dependencies with `add` or `remove`.

## Ruff, ty or pytest not found

```console
$ pyfaro check

== pyfaro check summary ==
format: unavailable (ruff not found in the project environment, VIRTUAL_ENV, or PATH; run `pyfaro add --dev ruff` then `pyfaro sync`)
lint: unavailable (ruff not found in the project environment, VIRTUAL_ENV, or PATH; run `pyfaro add --dev ruff` then `pyfaro sync`)
type: unavailable (ty not found in the project environment, VIRTUAL_ENV, or PATH; run `pyfaro add --dev ty` then `pyfaro sync`)
check: failed (format, lint, type)
```

```console
$ pyfaro tools ruff
error: ruff: not found; install ruff in the project environment (.venv by default), activate an environment, or add it to PATH
```

Tools are looked up in the project's `.venv`, then `VIRTUAL_ENV`, then `PATH`.
Add them to the `dev` group and sync:

```sh
pyfaro add --dev ruff ty pytest
pyfaro sync
```

Projects made with `pyfaro create` already have them. `pyfaro tools` lists
what was found and where.

## uv not found or too old

```console
$ pyfaro sync --uv /nonexistent/uv
error: could not find uv at /nonexistent/uv; fix --uv <path> or install uv (https://docs.astral.sh/uv/getting-started/installation/, or `curl -LsSf https://astral.sh/uv/install.sh | sh`)
```

```console
$ pyfaro sync
error: could not find uv on PATH; install uv (https://docs.astral.sh/uv/getting-started/installation/, or `curl -LsSf https://astral.sh/uv/install.sh | sh`) or pass --uv <path>
```

```console
$ pyfaro sync --uv ./old-uv
error: unsupported uv version: uv 0.8.0; use uv >=0.9.26 (tested: 0.9.26) by running `uv self update`, reinstalling uv (https://docs.astral.sh/uv/getting-started/installation/) or selecting another with --uv <path>
```

Install or upgrade uv (0.9.26 or newer) and make sure it is on `PATH`, or pass
`--uv <path>` to any command that talks to uv. `pyfaro doctor` reports the same
line as `ERROR: uv: …`. Pyfaro never installs uv for you.

## Stale lock file

```console
$ pyfaro sync --locked
Resolved 15 packages in 10ms
The lockfile at `uv.lock` needs to be updated, but `--locked` was provided. To update the lockfile, run `uv lock`.
error: uv sync failed (exit status: 1); see uv's output above
```

`pyproject.toml` changed without the lock being updated. Run `pyfaro sync`
(without `--locked`) or `uv lock` locally, and commit `uv.lock`.

## Invalid `[tool.pyfaro]` configuration

```console
$ pyfaro sync
error: invalid ~/projects/demo/pyproject.toml: tool.pyfaro.schema-version must be the integer 1
```

`doctor` reports the same problem as a blocking finding:

```console
$ pyfaro doctor
OK: project: ~/projects/demo
ERROR: configuration: invalid ~/projects/demo/pyproject.toml: tool.pyfaro.schema-version must be the integer 1; correct pyproject.toml
...
error: doctor found 1 blocking issue(s); follow the actions above and run pyfaro doctor again
```

The only supported `schema-version` is `1`, and unknown keys under
`[tool.pyfaro]` are rejected. A duplicate `[tool.pyfaro]` table is reported
as a TOML parse error with the line number.

## Interpreter does not satisfy `requires-python`

`doctor` reports
`Python 3.12.12 does not satisfy requires-python ">=3.13"`, and `sync` fails
with a similar message. Pin a compatible interpreter with
`pyfaro python pin`; see [Python versions](python-versions.md).

## The tools report failures

`lint`, `fmt`, `type` and `check` pass the tool's diagnostics straight
through:

```console
$ pyfaro lint
invalid-syntax: Expected a parameter or the end of the parameter list
 --> src/demo/bad.py:1:7
  |
1 | def f(:
  |       ^

Found 2 errors.
error: ruff failed (exit status: 1); see ruff's output above
```

These are problems in your code, not in Pyfaro. Fix them, or let
`pyfaro check --fix` repair what is safe to repair; see
[The quality gate](quality-gate.md).

## `pyfaro build` fails

`pyfaro build` runs `uv build`, so the explanation of a failed build is in uv's
output above the last line, and the exit status is uv's:

```console
$ pyfaro build
Building source distribution (uv build backend)...
  × Failed to build
  │ `~/projects/acme-widget`
  ╰─▶ Expected a Python module at: src/acme_widget/__init__.py
error: uv build failed (exit status: 2); see uv's output above
$ echo $?
2
```

The `2` is uv's, not a Pyfaro usage error. Read the lines above `error:` first.
The usual causes:

| Symptom | Cause and fix |
| --- | --- |
| `Expected a Python module at: src/<package>/__init__.py` | The `uv_build` backend wants the package named after the project in `src/`. Rename the directory, or set `[tool.uv.build-backend] module-name`. |
| A backend such as `hatchling` or `setuptools` cannot be downloaded, or the network was disabled | Backends other than `uv_build` are fetched into a build environment. Run without `--offline`, or warm uv's cache first. |
| `uv build succeeded but wrote no wheel (.whl) for …` | The backend wrote a file under another name. Check `[build-system]` and the project name. |
| `--clear would delete the project` (exit `2`) | `--out-dir` is the project or one of its parents. Choose a directory below or beside it. |
| No compatible Python | `build` passes `--no-python-downloads`. Install one with `pyfaro python install`. |

Rerun with [`-v`](#verbose-output) to see the exact `uv build` command line,
including the `--out-dir` it resolved, and copy it to reproduce the failure
without Pyfaro. See the [`build` guide](../commands/build.md).

## Tab completion does not work

`pyfaro completions <SHELL>` only prints a script; the shell must load it. Check
that the file is where your shell looks (the [`completions` guide](../commands/completions.md)
lists the places), that you opened a new shell afterwards, and that you
regenerate the script after upgrading Pyfaro, since an old script does not know
newer commands. In Zsh, the directory must be on `fpath` *before* `compinit`
runs, and a stale `~/.zcompdump*` cache can hide a new script; delete it and
restart the shell. Completion is static: it offers commands and flags, not the
names of your tasks.

## Verbose output

When a failure is not obvious, rerun the command with `-v` (or `--verbose`).
Before launching anything, Pyfaro prints to standard error which tools it
resolved, with their versions and paths, and every command line it executes:

```console
$ pyfaro -v lint
pyfaro lint: running ~/projects/demo/.venv/bin/ruff --version
pyfaro lint: using ruff 0.16.10 at ~/projects/demo/.venv/bin/ruff
pyfaro lint: running ~/projects/demo/.venv/bin/ruff check
All checks passed!
```

The flag works before the subcommand (`pyfaro -v sync`) and after it
(`pyfaro sync --verbose`) for every command except `test`. `pyfaro test`
forwards everything after `test` to pytest, so `pyfaro test -v` makes pytest
verbose; use `pyfaro -v test` for Pyfaro's trace.

The trace includes:

- `using <tool version> at <path>`: uv for every command that runs it, Ruff
  for `lint`, `fmt` and `check`, ty for `type` and `check`, pytest for `test`.
  This shows which installation wins when you have several.
- `running <command line>`: every process, including uv's version probes, task
  commands and the three steps of `check`. Copy a line to reproduce it by hand.
- `working directory <path>`: when a process does not run in the project root.
- `environment variables set: A, B`: the names, never the values, of the
  variables Pyfaro sets, for example a task's `env`.

```console
$ pyfaro -v run build
pyfaro run: running ~/.local/bin/uv --version
pyfaro run: using uv 0.9.26 at ~/.local/bin/uv
pyfaro run: running ~/.local/bin/uv --no-python-downloads --no-progress run --project ~/projects/demo -- python build.py
pyfaro run: environment variables set: BUILD_MODE, TOKEN
```

Credentials are masked, so you can paste the output into an issue: the
password of `https://user:secret@host/simple` prints as
`https://user:****@host/simple`, and a lone token as `https://****@host`. This
applies to the trace, to Pyfaro's error messages and to the `doctor` report.
The output of the tools themselves streams through untouched. See
[Verbose output and redaction](https://github.com/pyfaro/pyfaro/blob/main/docs/diagnostics.md)
for the exact rules.

## Still stuck?

- Run `pyfaro doctor` and `pyfaro tools` and read every line, then rerun the
  failing command with [`-v`](#verbose-output) (`pyfaro -v <command>`).
- Check [Platform support](../concepts/platform-support.md): `init --apply` is
  Linux-only for now.
- Open an issue at <https://github.com/pyfaro/pyfaro/issues> with the output
  of `pyfaro --version`, `pyfaro doctor` and the failing command with `-v`.
