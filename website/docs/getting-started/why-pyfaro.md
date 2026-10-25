# Why Pyfaro

The first question is fair: *why not just use uv?* This page answers it with
commands you can run, shows what the tools people already use for the same job
look like next to Pyfaro, and lists what Pyfaro does not do (yet).

Pyfaro adds no resolver, lock file format, linter or test engine. It is one
command line over [uv](https://docs.astral.sh/uv/),
[Ruff](https://docs.astral.sh/ruff/), [ty](https://docs.astral.sh/ty/) and
[pytest](https://docs.pytest.org/), with a few conventions and some
supervision on top. If you only need uv, keep using uv.

!!! note "About the snippets"
    Every Pyfaro snippet runs with `0.5.0-beta.1`, the latest published
    version. The other tools were run in the versions listed in
    [How the comparison was made](#how-the-comparison-was-made). Tabs are
    ordered the same way everywhere: Pyfaro first.

## The short version

- **One entry point with conventions.** `create`, `sync`, `check`, `test`,
  `dev` and `build` mean the same thing in every Pyfaro project, so a new
  contributor, a CI job and a git hook run the same commands.
- **A quality gate that needs no setup.** `pyfaro check` runs the format
  check, the lint and the type check, always runs all three, and prints one
  summary.
- **Scaffolds that are ready to use.** `pyfaro create` writes a project with
  tests, Ruff and pytest settings and the dev tools already declared.
- **A supervised dev server.** `pyfaro dev` runs Uvicorn with reload and, on
  Ctrl-C, stops the whole process tree.
- **Nothing hidden.** `check`, `test` and `lint` never sync or install behind
  your back, and everything stays in `pyproject.toml` and `uv.lock`.

What it costs: another tool to install, and less flexibility than a task
runner. [What Pyfaro does not do (yet)](#what-pyfaro-does-not-do-yet) is
listed below.

## At a glance

| | Creates a project | Locks and syncs | Named tasks | One quality gate |
| --- | --- | --- | --- | --- |
| **Pyfaro** | `create lib`, `cli`, `fastapi` | through uv (`uv.lock`) | `[tool.pyfaro.tasks]`, one command each | `check` |
| **uv** | `uv init` | yes (`uv.lock`) | not documented | no |
| **uv + Makefile or just** | `uv init` | through uv | recipes in a separate file | the recipe you write |
| **poethepoet** | no, it runs tasks | no, it runs tasks | `[tool.poe.tasks]`, with sequences and parallel tasks | the sequence you write |
| **Hatch** | `hatch new` | per environment; locks are opt-in (`pylock.toml`) | scripts per environment | `hatch check` |
| **PDM** | `pdm init` | its own resolver and `pdm.lock` | `[tool.pdm.scripts]`, with composite scripts | the composite script you write |

Read the table as "what is built in or one line of configuration away", not as
a ranking: every task runner below can run the same commands Pyfaro runs.

## Create a project

=== "Pyfaro"

    ```sh
    pyfaro create lib acme-lib
    cd acme-lib
    pyfaro sync
    ```

    Writes a `src/` layout with a typed package, a passing test, Ruff and
    pytest settings, and `pytest`, `ruff` and `ty` in the `dev` dependency
    group, pinned to the versions Pyfaro is tested with. `cli` and `fastapi`
    templates exist too.

=== "uv"

    ```sh
    uv init --lib acme-lib
    cd acme-lib
    uv add --dev pytest ruff ty
    ```

    `uv init --lib` writes a `src/` layout and a build backend. It writes no
    tests and no dev tools, so you add them and decide where their settings
    live.

=== "uv + just"

    ```sh
    uv init --lib acme-lib
    cd acme-lib
    uv add --dev pytest ruff ty
    # then write a justfile, see the sections below
    ```

=== "poethepoet"

    ```sh
    uv init --lib acme-lib
    cd acme-lib
    uv add --dev pytest ruff ty poethepoet
    # then add [tool.poe.tasks] to pyproject.toml
    ```

    poethepoet is a task runner: it does not create projects.

=== "Hatch"

    ```sh
    hatch new acme-lib
    ```

    Writes a `src/` layout with `tests/`, the `hatchling` build backend, a
    `types` environment and coverage settings.

=== "PDM"

    ```sh
    mkdir acme-lib && cd acme-lib
    pdm init -n --lib
    pdm add -dG dev pytest ruff
    ```

## Adopt an existing project

=== "Pyfaro"

    ```sh
    cd old-project
    pyfaro init            # preview, writes nothing
    pyfaro init --apply    # Linux only in 0.5.0-beta.1
    pyfaro add --dev ruff ty pytest
    pyfaro sync
    ```

    `init` keeps your dependencies, `[tool.uv]`, `[tool.ruff]` and comments,
    and adds only a `[tool.pyfaro]` table and a lock file. It does not add
    dev tools: until you add them, `pyfaro check` reports them as
    unavailable. On macOS and Windows `0.5.0-beta.1` previews but does not
    apply.

=== "uv"

    ```sh
    cd old-project
    uv init --bare                       # only if there is no pyproject.toml
    uv add -r requirements.txt
    uv sync
    ```

    A project that already has a `[project]` table needs nothing: run
    `uv sync`.

=== "uv + just"

    The same as uv, then add a `justfile`. Nothing else in the project
    changes.

=== "poethepoet"

    ```sh
    uv add --dev poethepoet
    # then add [tool.poe.tasks] to pyproject.toml
    ```

=== "Hatch"

    ```sh
    cd old-project
    hatch new --init
    ```

    `--init` initializes an existing project.

=== "PDM"

    ```sh
    cd old-project
    pdm import requirements.txt
    ```

    `pdm import` reads `requirements.txt`, Pipfile, Poetry, Flit and
    `setup.py` metadata.

## The quality gate

Format check, lint and type check, all of them reported even when one fails.

=== "Pyfaro"

    ```console
    $ pyfaro check
    unformatted: File would be reformatted
    …
    == pyfaro check summary ==
    format: failed (exit 1)
    lint: failed (exit 1)
    type: failed (exit 1)
    check: failed (format, lint, type)
    ```

    `check` runs `ruff format --check`, `ruff check` and `ty check`, always
    runs all three, and exits 1 if any failed. It writes files only with
    `--fix`, which applies Ruff's safe fixes and formats last. It needs no
    configuration: Ruff reads `[tool.ruff]` and ty reads `[tool.ty]`.

=== "uv"

    ```sh
    uv run ruff format --check && uv run ruff check && uv run ty check
    ```

    With `&&` the chain stops at the first failure, so a formatting problem
    hides the lint and type errors until you fix it. Running the three as
    separate commands shows everything, but then you read three exit codes.

=== "uv + just"

    ```just title="justfile"
    # Format check, lint and type check
    check:
        uv run ruff format --check
        uv run ruff check
        uv run ty check
    ```

    ```sh
    just check
    ```

    A recipe stops at the first failing line, so it behaves like the `&&`
    chain. `just --list` shows the recipe and its comment.

=== "poethepoet"

    ```toml title="pyproject.toml"
    [tool.poe.tasks]
    fmt-check = "ruff format --check"
    lint = "ruff check"
    types = "ty check"
    check = { sequence = ["fmt-check", "lint", "types"], ignore_fail = "return_non_zero" }
    ```

    ```sh
    uv run poe check
    ```

    With `ignore_fail = "return_non_zero"` all three tasks run and poethepoet
    exits non-zero if any failed, which is the same behavior as `pyfaro
    check`. Without it a sequence stops at the first failure.

=== "Hatch"

    ```sh
    hatch check
    ```

    `hatch check` is Hatch's own gate: it runs a lint stage, a format stage
    and a type-check stage (Pyrefly in Hatch 1.18.1), and takes `--fix`. In
    1.18.1 it stopped at the first failing stage.

=== "PDM"

    ```toml title="pyproject.toml"
    [tool.pdm.scripts]
    fmt-check = "ruff format --check"
    lint = "ruff check"
    types = "ty check"
    check = { composite = ["fmt-check", "lint", "types"], keep_going = true }
    ```

    ```sh
    pdm run check
    ```

    `keep_going = true` continues after a failure. Without it a composite
    script stops at the first failure.

## Tests

=== "Pyfaro"

    ```sh
    pyfaro test -k parse
    ```

    Runs pytest with the project environment's Python and forwards every
    argument. It does not sync first: run `pyfaro sync` when dependencies
    change.

=== "uv"

    ```sh
    uv run pytest -k parse
    ```

    `uv run` makes sure the environment is up to date before it runs the
    command. Add `--locked` or `--frozen` to be strict.

=== "uv + just"

    ```just title="justfile"
    test *args:
        uv run pytest {{args}}
    ```

    ```sh
    just test -k parse
    ```

=== "poethepoet"

    ```toml title="pyproject.toml"
    [tool.poe.tasks]
    test = "pytest"
    ```

    ```sh
    uv run poe test -k parse
    ```

=== "Hatch"

    ```sh
    hatch test -k parse
    ```

    `hatch test` runs pytest in Hatch's test environment matrix, with
    dependencies such as coverage included.

=== "PDM"

    ```toml title="pyproject.toml"
    [tool.pdm.scripts]
    test = "pytest"
    ```

    ```sh
    pdm run test -k parse
    ```

## A development server

=== "Pyfaro"

    ```toml title="pyproject.toml"
    [tool.pyfaro.dev]
    preset = "fastapi"
    entry-point = "acme_api.main:app"
    ```

    ```console
    $ pyfaro dev
    pyfaro dev: starting acme_api.main:app with uvicorn (reload on) at http://127.0.0.1:8000 - press Ctrl-C to stop
    INFO:     Uvicorn running on http://127.0.0.1:8000 (Press CTRL+C to quit)
    ^C
    pyfaro dev: stopped
    $ echo $?
    130
    ```

    `pyfaro create fastapi` writes that table for you. Before it launches,
    `dev` checks that `uvicorn` is declared and that the port is free. On
    Ctrl-C it stops the whole process tree (Uvicorn's reloader and its
    worker); on Unix it waits about five seconds for a clean shutdown and ends
    what is left. It then exits 130.

=== "uv"

    ```sh
    uv run uvicorn acme_api.main:app --reload
    ```

=== "uv + just"

    ```just title="justfile"
    dev:
        uv run uvicorn acme_api.main:app --reload
    ```

=== "poethepoet"

    ```toml title="pyproject.toml"
    [tool.poe.tasks]
    dev = "uvicorn acme_api.main:app --reload"
    ```

    ```sh
    uv run poe dev
    ```

=== "Hatch"

    ```toml title="pyproject.toml"
    [tool.hatch.envs.default.scripts]
    dev = "uvicorn acme_api.main:app --reload"
    ```

    ```sh
    hatch run dev
    ```

=== "PDM"

    ```toml title="pyproject.toml"
    [tool.pdm.scripts]
    dev = "uvicorn acme_api.main:app --reload"
    ```

    ```sh
    pdm run dev
    ```

Every alternative runs the same Uvicorn command. What `pyfaro dev` adds is the
preflight checks, the message that says what it started, and a stop that
leaves no process behind. See [`pyfaro dev`](../commands/dev.md) for the exact
behavior, including Windows.

## Named tasks

Task runners are what this section is about, and the dedicated tools are more
capable than Pyfaro here (see [what Pyfaro does not do
yet](#what-pyfaro-does-not-do-yet)).

=== "Pyfaro"

    ```toml title="pyproject.toml"
    [tool.pyfaro.tasks]
    fix = ["ruff", "check", "--fix", "."]
    serve-docs = { command = ["python", "-m", "http.server", "8001"], cwd = "docs", description = "Serve the docs folder" }
    ```

    ```sh
    pyfaro run fix
    pyfaro run fix -- --select I
    ```

    A task is one command in the project environment. It never goes through
    a shell (`shell = "..."` is an explicit opt-in), so it behaves the same on
    Linux, macOS and Windows, and arguments after `--` are appended. A task
    cannot depend on or call another task.

=== "uv"

    uv documents no named tasks: there is no `[tool.uv]` key for them.
    `uv run <command>` runs a command in the project environment, and
    `[project.scripts]` defines entry points that your package installs,
    which is a different feature.

=== "uv + just"

    ```just title="justfile"
    fix:
        uv run ruff check --fix .
    ```

    ```sh
    just fix
    ```

    just recipes can have dependencies, parameters and a `[parallel]`
    attribute. A recipe line is a shell line (`sh` by default; `windows-shell`
    and `set shell` change it), so recipes meant for every platform need care.

=== "poethepoet"

    ```toml title="pyproject.toml"
    [tool.poe.tasks]
    fix = "ruff check --fix ."
    ```

    ```sh
    uv run poe fix
    ```

    poethepoet also has `shell`, `script`, `expr`, `sequence`, `parallel`,
    `switch` and `ref` tasks.

=== "Hatch"

    ```toml title="pyproject.toml"
    [tool.hatch.envs.default.scripts]
    fix = "ruff check --fix ."
    ```

    ```sh
    hatch run fix
    ```

    Scripts belong to an environment, can reference one another, and an array
    runs commands in sequence.

=== "PDM"

    ```toml title="pyproject.toml"
    [tool.pdm.scripts]
    fix = "ruff check --fix ."
    ```

    ```sh
    pdm run fix
    pdm run --list
    ```

    PDM scripts can be `cmd`, `shell`, `call` or `composite`.

`pyfaro run` with no argument lists the tasks, and Pyfaro validates `[tool.pyfaro.tasks]` before running anything. See
[`pyfaro run`](../commands/run.md#named-tasks).

## Build

=== "Pyfaro"

    ```sh
    pyfaro build
    ```

    ```console
    built dist/acme_lib-0.1.0.tar.gz
    built dist/acme_lib-0.1.0-py3-none-any.whl
    ```

    Runs `uv build` with the build backend your `pyproject.toml` declares and
    prints the files it created on standard output.

=== "uv"

    ```sh
    uv build
    ```

=== "uv + just"

    ```just title="justfile"
    build:
        uv build
    ```

=== "poethepoet"

    ```toml title="pyproject.toml"
    [tool.poe.tasks]
    build = "uv build"
    ```

=== "Hatch"

    ```sh
    hatch build
    ```

=== "PDM"

    ```sh
    pdm build
    ```

Here Pyfaro is a thin pass-through: `pyfaro build` and `uv build` do the same
work.

## Continuous integration

=== "Pyfaro"

    ```yaml
    - run: pyfaro sync --locked
    - run: pyfaro check
    - run: pyfaro test
    ```

    The job also installs uv and Pyfaro; see
    [Continuous integration](../workflows/continuous-integration.md) for a
    complete workflow.

=== "uv"

    ```yaml
    - run: uv sync --locked
    - run: uv run ruff format --check
    - run: uv run ruff check
    - run: uv run ty check
    - run: uv run pytest
    ```

    Separate steps show which one failed in the CI interface. A failing step
    stops the job unless you add `if: always()` to the later ones.

=== "uv + just"

    ```yaml
    - run: uv sync --locked
    - run: just check
    - run: just test
    ```

    The job also installs just.

=== "poethepoet"

    ```yaml
    - run: uv sync --locked
    - run: uv run poe check
    - run: uv run poe test
    ```

=== "Hatch"

    ```yaml
    - run: hatch check
    - run: hatch test
    ```

    The job installs Hatch, which creates its environments on first use.

=== "PDM"

    ```yaml
    - run: pdm install --check
    - run: pdm run check
    - run: pdm run test
    ```

    `pdm install --check` refuses a missing or outdated lock file.

The Pyfaro job installs one more tool than the uv job. In exchange, the three
commands stay the same on every project and on a laptop.

## When another tool is the better fit

- **You only need to run a few commands.** uv alone, or uv with a `justfile`.
  Pyfaro would add a tool without adding much.
- **You need dependencies between tasks, parallel tasks or file-based
  caching today.** just, poethepoet or PDM composite scripts. Pyfaro tasks
  are single commands.
- **You already use Hatch or PDM and are happy with it.** Both are complete
  project managers with their own environments and lock files. Pyfaro does not
  read their environments, scripts or `pdm.lock`; it works through uv and
  `uv.lock`.
- **You want a different linter, formatter or type checker.** Pyfaro runs Ruff
  and ty and does not swap them. A task runner can run anything.

Pyfaro fits when you start a Python project on uv, Ruff, ty and pytest and
want one set of commands that is the same everywhere, with a quality gate and
a dev server that need no setup.

## What Pyfaro does not do (yet)

These are the gaps that matter when you compare. They are planning targets,
not promises: milestones can move.

- **No task graph, parallelism or cache.** A task is one command; a task
  cannot depend on or call another, and `pyfaro run` runs one task at a time.
  A dependency-aware task graph, parallel execution and an opt-in local cache
  are planned for the
  [v1.1.0 milestone](https://github.com/pyfaro/pyfaro/milestone/7).
- **No workspaces.** Pyfaro works on one project at a time. uv workspace
  support is planned for the same
  [v1.1.0 milestone](https://github.com/pyfaro/pyfaro/milestone/7).
- **No JSON output.** Output is for people. A versioned JSON output is
  planned for the
  [v1.2.0 milestone](https://github.com/pyfaro/pyfaro/milestone/8).
- **It does not replace uv, Ruff, ty or pytest.** It calls them and passes
  their output and exit status through. It needs uv 0.9.26 or newer, and it
  never installs uv or a Python interpreter for you.
- **No `publish` command.** `pyfaro build` builds the distributions and
  stops there.
- **`init --apply` is Linux-only in `0.5.0-beta.1`.** On macOS and Windows,
  `init` previews and `create` works.
- **Beta.** Interfaces may still change before 1.0. See the
  [project status](https://github.com/pyfaro/pyfaro#project-status).

The [roadmap](https://github.com/pyfaro/pyfaro#roadmap) links the
[roadmap board](https://github.com/orgs/pyfaro/projects/1) and all
[milestones](https://github.com/pyfaro/pyfaro/milestones).

## How the comparison was made

The Pyfaro snippets were run on Linux with Pyfaro `0.5.0-beta.1`
(`0.5.0b1` from PyPI) and uv 0.9.26, on projects whose templates pin Ruff
0.16.10, ty 0.0.85 and pytest 9.1.1. The other tools were run on Linux with
uv 0.9.26 in these versions: Hatch 1.18.1, PDM 2.29.3, poethepoet 0.49.0 and
just 1.59.0, with the newest Ruff (0.17.2) and ty (0.0.91) that uv installed
for them. Their behavior was also checked
against their current documentation (uv's was read on 2026-10-25, when the
newest uv release was 0.13.5, newer than the 0.9.26 that Pyfaro is tested
with): [uv](https://docs.astral.sh/uv/concepts/projects/run/),
[just](https://just.systems/man/en/),
[poethepoet](https://poethepoet.natn.io/tasks/index.html),
[Hatch](https://hatch.pypa.io/latest/config/environment/overview/) and
[PDM](https://pdm-project.org/latest/usage/scripts/). If something here is out
of date or unfair to another tool, please
[open an issue](https://github.com/pyfaro/pyfaro/issues).

## Next steps

- [Install Pyfaro](installation.md) and follow the [first steps](first-steps.md).
- Browse the [features](features.md) and [commands](../commands/index.md).
- See the [quality gate](../workflows/quality-gate.md) and
  [CI](../workflows/continuous-integration.md) workflows.
