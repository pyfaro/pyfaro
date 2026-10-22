# Exit codes

Pyfaro distinguishes its own failures from the failures of the tools it runs.
Errors are written to standard error, starting with `error:`.

| Code | Meaning |
| --- | --- |
| `0` | Success. Also `--help` and `--version`. |
| `1` | A Pyfaro failure, a failing `check`, or a `doctor` report with an `ERROR`. |
| `2` | Invalid usage: an unknown flag, a missing argument. |
| tool's code | The delegated tool's own exit code, forwarded unchanged. |
| `130` | Cancelled with `Ctrl+C`. |

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

A child process that ends without a numeric status (killed by a signal, for
instance) also gives 1, with a diagnostic.

## Usage errors: 2

Invalid command-line usage exits with 2:

```console
$ pyfaro sync --bogus
error: unexpected argument '--bogus' found
...
$ echo $?
2
```

## Forwarded codes

When a command runs a single tool, the tool's code is the command's code:

```console
$ pyfaro test -k nomatch
...
============================ 1 deselected in 0.01s =============================
error: pytest failed (exit status: 5); see pytest's output above
$ echo $?
5
```

pytest's codes keep their meaning (1 failures, 2 interrupted, 5 no tests
collected, and so on), so scripts can tell them apart. The same holds for
`run`, `sync`, `add`, `remove`, `lint`, `fmt`, `type` and the `python`
commands, which return what uv or Ruff or ty returned.

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

## Cancellation: 130

On `Ctrl+C`, Pyfaro stops the process tree it started, waits for it to
exit, and returns 130. If cleanup cannot be confirmed, it reports an error.

## Next steps

- [Continuous integration](../workflows/continuous-integration.md) shows how to
  use these codes in a pipeline.
- [The quality gate](../workflows/quality-gate.md).
