# Exit codes

Pyfaro's exit codes are a stable contract for the public beta (`0.5.0-beta.1`
and later). Scripts and CI can rely on the meanings below; a change to them is
a breaking change that needs a changelog entry. The wording of messages is not
part of the contract, except for the prefixes described under
[Where diagnostics go](#where-diagnostics-go).

| Code | Meaning |
| --- | --- |
| `0` | Success. Also `--help`, `--version`, a `doctor` report with warnings only and a `tools` listing. |
| `1` | A Pyfaro failure; a failing `check`; a `doctor` report with an `ERROR`; a delegated tool that ended without a numeric status. |
| `2` | Invalid usage: a Clap error or `Error::Usage`. |
| other | The exit status of a single delegated tool, forwarded unchanged. |
| `130` | Cancelled with Ctrl-C (or the console's equivalent on Windows). |

## `1`: Pyfaro's own failures

Exit `1` covers everything Pyfaro detects itself, before or around running a
tool:

- no project found, `--project` that is not a project, an invalid
  `pyproject.toml`, `[tool.pyfaro]`, `[tool.pyfaro.tasks]` or
  `[tool.pyfaro.dev]`;
- uv or Ruff, ty or pytest not found, not executable, unsupported or unable to
  report a version (`pyfaro tools <tool>` included);
- a tool that cannot be started, or a rejected task definition;
- `pyfaro dev` refusing to start (missing table, undeclared Uvicorn, busy
  port);
- process cleanup that could not be confirmed after a cancellation, and
  rollback failures of `init --apply`, `add` and `remove`;
- `pyfaro check` when any of its three steps failed, whatever codes the tools
  returned (all steps always run), and a step whose tool is missing;
- `pyfaro doctor` when it reports at least one `ERROR`. A failed tool version
  probe is a doctor finding, so its exit status is not forwarded;
- a delegated tool that ended **without a numeric exit status**, for example
  because a signal killed it: `error: <tool> failed (signal: 9 (SIGKILL))` and
  exit `1`. A child that is killed by a signal inside `uv run` is different:
  uv reports it as `128 + signal`, and that number is forwarded (see below).

## `2`: usage errors

Clap rejects the command line: an unknown flag or subcommand, a missing
required argument, conflicting options such as `--locked --frozen`, or no
arguments at all (the help is printed to standard error). `Error::Usage` is the
same class for a command line that Clap accepted but Pyfaro cannot act on:
`pyfaro run` with neither a task nor `-- <program>`, and `pyfaro run <name>`
outside a project, which is almost always a forgotten `--`; and
`pyfaro build --clear` with an output directory that is or contains the project.

## Forwarded codes

A command that runs **one** tool returns that tool's numeric status unchanged.

| Command | Status returned |
| --- | --- |
| `sync`, `build`, `add`, `remove`, `python list`, `python install`, `python pin` | uv's. |
| `init --apply` | uv's, from `uv init` or `uv lock`. |
| `run` (a program or a task) | The program's, through `uv run`; uv exits `128 + signal` for a program killed by a signal. A uv failure such as a missing program is uv's own code. |
| `dev` | The process's, through `uv run` (`pyfaro dev: <runner> exited with status N`). `0` stays silent. |
| `lint`, `fmt`, `type` | Ruff's or ty's. |
| `test` | pytest's: `1` failures, `2` interrupted, `3` internal error, `4` usage error, `5` nothing collected. |

When `uv --version` itself fails, the delegating commands above return uv's
status for that failed probe as well. An unsupported uv version is a Pyfaro
failure (`1`).

A forwarded code can be any number, including `1` and `2`, so a number alone
does not tell a tool failure from a Pyfaro failure. A forwarded failure prints
`error: <tool> failed (<status>); see <tool>'s output above` on standard error
(`pyfaro dev: ...` for `dev`), after the tool's own output.

Commands that run several tools or report instead of delegating do not
forward: `check` (`1`), `doctor` (`1`), `tools` (`0`, or `1` for
`pyfaro tools <tool>` when the tool is missing or broken), `create` (`1` on
failure; it runs no tool).

## `130`: cancellation

On Ctrl-C every command that runs a child stops the whole process tree it
started, waits for it, and exits `130`. `pyfaro dev` prints
`pyfaro dev: stopped` first; the other commands print
`error: operation cancelled`. If the tree cannot be confirmed gone, the exit
status is `1` with an error that says so. On Windows `pyfaro dev` waits a few
seconds for a tree that ends by itself (a Uvicorn reload) before treating a
Ctrl-C as a stop.

## Where diagnostics go

Everything Pyfaro itself prints on failure goes to **standard error**:

- `error: <message>` for a failure reported by `main`;
- `pyfaro <command>: <message>` lines, used by `dev`, by
  [`--verbose`](diagnostics.md) and by notes such as `pyfaro: running the
  program ...`;
- the `check` summary.

Reports that are a command's output stay on standard output: the `doctor`
lines, `tools`, `python list`. Child processes write to the same streams as
they would without Pyfaro. Pyfaro passes them through unchanged and does not
rewrite them, so [redaction](diagnostics.md#redaction) only applies to text
Pyfaro prints itself.
