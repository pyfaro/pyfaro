# ADR 0002: Named tasks and safe invocation

- Status: Accepted
- Date: 2026-10-23
- Related backlog IDs: D05, D06
- Builds on: [ADR 0001](0001-project-architecture.md)

## Context

`pyfaro run -- <program>` runs a program in the project environment through
`uv run`, with explicit arguments and no shell. Projects also want short names
for the commands they repeat (`lint-all`, `serve-docs`), recorded next to the
rest of the project configuration, and a few of those commands genuinely need a
shell (a pipe, `&&`).

Two risks come with that. First, a name given to `pyfaro run` could mean a
configured task or a program on `PATH`, and the meaning must never change
silently when a task is added or renamed. Second, a shell is a quoting and
injection hazard whose rules differ between Unix and Windows. ADR 0001 already
requires that shell tasks need explicit opt-in and separate design (D06); this
record is that design, and the implementation (D05) follows it.

## Decision

### Configuration

Tasks live in `[tool.pyfaro.tasks]` in `pyproject.toml`. `schema-version`
stays `1`: the table is additive, and unknown keys anywhere under
`[tool.pyfaro]` remain errors.

```toml
[tool.pyfaro.tasks]
lint-all = ["ruff", "check", "."]
serve-docs = { command = ["python", "-m", "http.server", "8001"], cwd = "docs", env = { PYTHONUNBUFFERED = "1" }, description = "Serve the docs folder" }
changes = { shell = "git log --oneline | head -n 5" }
```

- A task name matches `^[A-Za-z0-9][A-Za-z0-9_.-]*$` and has at most 64
  characters.
- A task is either a non-empty array of non-empty strings (the argument vector,
  shorthand for `command`), or a table with exactly one of `command` (the same
  kind of array) or `shell` (a non-empty string), plus optional `cwd` (string),
  `env` (table of strings) and `description` (string). Unknown task keys, both
  or neither of `command` and `shell`, and wrong types are errors.
- `cwd` is a path relative to the project root. Absolute paths and paths whose
  `..` components climb above the root are rejected. The directory must exist
  when the task runs, and it must still resolve inside the project root after
  symlinks are followed. The default is the project root, not the directory
  `pyfaro` was started from.
- `env` keys are non-empty and contain neither `=` nor NUL; values contain no
  NUL. The table is merged over the inherited environment and task values win.
  There is no interpolation: `$VAR` and `%VAR%` stay literal in command tasks.
- All of this is validated when the configuration is loaded, so a malformed task
  fails with the `pyproject.toml` path and the dotted key before anything is
  launched. The Windows script check below depends on the platform and
  therefore runs when that task is launched.

### Invocation and resolution

- `pyfaro run <task> [-- <extra>...]` (a name before any `--`) always means a
  task. An unknown name is an error (exit 1) that lists the defined tasks and
  says `to run a program, use pyfaro run -- <name>`.
- `pyfaro run -- <program> [args...]` always means a program and behaves exactly
  as before, including the `.py` script handling. If `<program>` equals the name
  of a defined task, Pyfaro prints a note to stderr before launching that says
  the program is being run and how to run the task instead. It does not stop.
- With neither a task nor `--`, the command is a usage error (exit 2) that also
  lists the defined tasks.

Resolution is decided by syntax, not by looking at the file system or `PATH`.
The meaning of a command line depends only on whether `--` precedes the name.

### Execution

Command tasks run `uv run --project <root> [--locked | --frozen] -- <argv...>
<extra...>`. The uv process gets the task's cwd as its working directory and the
task's `env` on top of the inherited environment, so both reach the child. The
argument vector is passed as separate `OsString`s: nothing is globbed, expanded
or re-split by Pyfaro. Output streaming, stdin handling, process-tree
containment, numeric exit-code forwarding and cancellation (130) share the code
path of the program form.

Shell tasks are the only place a shell is used, and only because the task says
`shell = ...`:

- Unix: `sh -c <script>`.
- Windows: `cmd.exe /d /s /c <script>`.
- The script is the single, final argument. Extra arguments after `--` are
  rejected (exit 1), because appending them would be injection by
  concatenation.
- On Windows, a script that contains `"` is rejected before launch. `cmd.exe`
  does not parse the command line with the rules used to quote arguments, so a
  quote cannot be carried through argv quoting without changing its meaning.
  The error says so and suggests a command task or a script file.

The shell still runs through `uv run`, so tools installed in the project
environment are on its `PATH`.

### Out of scope

Task dependencies, tasks calling tasks, parallelism, caching and a
`pyfaro tasks list` command are deferred to v1.1.0 together with the task graph
announced in ADR 0001.

## Alternatives considered

- **Run every task through the shell.** Familiar and compact, but it makes
  quoting platform-dependent and turns every `{extra}` into an injection point.
  Rejected; the shell is a per-task opt-in.
- **Detect collisions by looking at `PATH` or the file system**, so that `pyfaro
  run name` picks a program when one exists. The meaning of a command would
  change when a file appears or a tool is installed, which is the silent
  collision this design exists to prevent. Rejected.
- **Make a task shadow the program silently**, or error on every collision.
  Silent shadowing is the hazard above. Failing every `run -- <name>` that
  matches a task would break scripts for a naming accident. A note keeps the
  program form stable and still tells the reader.
- **A required subcommand (`pyfaro task <name>`)** avoids ambiguity but doubles
  the vocabulary for the common case. The `--` rule already exists and is easy
  to teach.
- **Variable interpolation (`${VAR}`) in command tasks.** It reintroduces a
  second expansion language and platform differences. Users who need expansion
  can opt into a shell task.
- **Quote or escape `"` on Windows shell tasks.** There is no escaping that is
  correct for both argv quoting and `cmd.exe`'s parser in every case. A clear
  rejection is safer than a script that quietly does something else.
- **Canonicalize `cwd` lexically only.** A symlink inside the project could
  still point outside. The resolved directory is checked after symlinks too.

## Consequences

- Adding or removing a task never changes the meaning of an existing
  `pyfaro run -- ...` command line; at most it prints a note.
- A first positional argument to `pyfaro run` that is not a task name is now an
  error with a hint, where before it was a usage error from the argument parser.
- Existing projects without `[tool.pyfaro.tasks]` are unaffected.
- A malformed `[tool.pyfaro.tasks]` table fails every command that loads the
  configuration, like any other invalid Pyfaro setting, before a tool is
  launched.
- The cross-platform behavior is covered by end-to-end tests on Linux, macOS and
  Windows: a literal-argument matrix for command tasks (spaces, quotes, `$`,
  `%`, `*`, `&`, `|`, `;`, Unicode, leading dashes, empty strings), real shell
  behavior for shell tasks, the Windows quote rejection, and cancellation.
- Extending the model later (dependencies, parallelism) can build on these
  rules; the invocation rules above do not need to change for it.
