---
description: "Pyfaro exit codes: how its own failures differ from the failures of the tools it runs, for scripts and CI."
---

# Exit codes

Pyfaro distinguishes its own failures from the failures of the tools it runs.
These exit codes are a **stable contract for the public beta**: scripts and CI
can rely on them, and a change is a breaking change announced in the
changelog. Message wording is not part of the contract.

| Code | Meaning |
| --- | --- |
| `0` | Success. Also `--help`, `--version`, and a `doctor` report with warnings only. |
| `1` | A Pyfaro failure, a failing `check`, a `doctor` report with an `ERROR`, or a tool that ended without a numeric exit status. |
| `2` | Invalid usage: an unknown flag, a missing argument, `pyfaro run` with nothing to run. |
| tool's code | The delegated tool's own exit code, forwarded unchanged. |
| `130` | Cancelled with `Ctrl+C`. |

Pyfaro's own diagnostics go to standard error: `error: ...` for failures and
`pyfaro <command>: ...` lines for `dev`, [`--verbose`](../workflows/troubleshooting.md#verbose-output)
and notes. Reports that are a command's output, such as `doctor` and `tools`,
stay on standard output. Child processes write where they always do and Pyfaro
does not rewrite their output.

## Pyfaro's own failures: 1

Exit code 1 covers problems Pyfaro detects before or around running a tool:
no project found, invalid `[tool.pyfaro]`, uv missing or too old, a tool not
found, a command that cannot be started.

```console
$ pyfaro sync
error: no pyproject.toml found from /tmp; use --project <PATH>
$ echo $?
1
```

A tool that ends without a numeric status (killed by a signal, for instance)
also gives 1, with a diagnostic such as `error: ruff failed (signal: 9
(SIGKILL))`. A program killed by a signal *inside* `pyfaro run` is different:
uv reports it as `128 + signal` (137 for `SIGKILL`) and that number is
forwarded.

## Usage errors: 2

Invalid command-line usage exits with 2. That covers Clap's errors (unknown
flags and subcommands, missing arguments, conflicting options, no arguments at
all) and the commands Pyfaro cannot act on, such as `pyfaro run` with neither a
task nor `-- <program>`:

```console
$ pyfaro sync --bogus
error: unexpected argument '--bogus' found
...
$ echo $?
2
```

## Forwarded codes

When a command runs a single tool, the tool's code is the command's code.
This holds for:

| Command | Code returned |
| --- | --- |
| `sync`, `build`, `add`, `remove`, `python` | uv's |
| `init --apply` | uv's |
| `run` (a program or a task) | the program's, through `uv run` |
| `dev` | the process's, through `uv run` |
| `lint`, `fmt`, `type` | Ruff's or ty's |
| `test` | pytest's |

For example:

```console
$ pyfaro test -k nomatch
...
============================ 1 deselected in 0.01s =============================
error: pytest failed (exit status: 5); see pytest's output above
$ echo $?
5
```

pytest's codes keep their meaning (1 failures, 2 interrupted, 5 no tests
collected, and so on), so scripts can tell them apart. A failed forwarded
command prints `error: <tool> failed (<status>); see <tool>'s output above`
after the tool's own output. If `uv --version` itself fails, the commands that
use uv return uv's status for that probe; an unsupported uv version is a
Pyfaro failure (1).

!!! note "A forwarded code can be 2 too"
    Because the tool's code is forwarded, 2 can also be uv's or pytest's own
    code. The `error:` line says which tool failed.

## `check`: aggregate 1

`pyfaro check` runs three steps, the format check, lint and type check, and
runs all of them even if one fails. It exits **1 if any step failed**,
whatever codes the tools returned, and 0 otherwise. The summary lists what
failed:

```console
$ pyfaro check
...
== pyfaro check summary ==
format: failed (exit 1)
lint: failed (exit 1)
type: failed (exit 1)
check: failed (format, lint, type)
```

## `doctor` and `tools`

- `doctor` exits 1 if it reports any `ERROR`. Warnings alone leave it at 0.
  A tool version probe that fails is a finding, not a forwarded code.
- `pyfaro tools` lists tools and exits 0. With a tool name, as in
  `pyfaro tools ruff`, a missing or broken tool is an error with exit 1.

## `dev`: the process's own code

`pyfaro dev` runs a long-lived process. If it exits with status `N`, Pyfaro
prints `pyfaro dev: <runner> exited with status N` and exits `N`; exit 0 is
silent. Ctrl+C prints `pyfaro dev: stopped` and exits 130. See
[`pyfaro dev`](../commands/dev.md).

## Cancellation: 130

On `Ctrl+C`, Pyfaro stops the process tree it started, waits for it to
exit, and returns 130 (`error: operation cancelled`, or `pyfaro dev: stopped`
for `dev`). If cleanup cannot be confirmed, it reports an error and exits 1.

## Next steps

- [Troubleshooting](../workflows/troubleshooting.md#verbose-output) shows
  `--verbose`, which prints the tool versions and commands behind a failure.
- [Continuous integration](../workflows/continuous-integration.md) shows how to
  use these codes in a pipeline.
- [The quality gate](../workflows/quality-gate.md).
