# ADR 0001: Integrate the Python toolchain

- Status: Accepted as the initial architectural direction
- Date: 2026-10-15
- Related backlog IDs: F01, F02, F05, F06, P01

## Context

Python development tools are capable but require users to compose commands,
environments, configuration, and diagnostics. Pyfaro offers an opinionated
workflow inspired by Vite+ while preserving the Python ecosystem's standards.
Rebuilding mature engines would multiply compatibility and security burdens.

## Decision

Pyfaro is a Rust integration layer. It owns CLI ergonomics, typed configuration,
project context, orchestration, diagnostics, developer workflows, and eventually
a task graph. It delegates Python installation, environments, dependency
resolution, dependency changes, and packaging to uv; lint and format to Ruff;
typing to ty; and testing to pytest.

There must be no second dependency solver, package manager, Python installer,
lockfile format, or replacement lint/type/test engine. `pyproject.toml` remains
project metadata; Pyfaro-specific settings belong in `[tool.pyfaro]`.
`uv.lock` belongs to uv. Preserve `[tool.ruff]`, `[tool.ty]`, pytest settings,
build backend configuration, and unrelated metadata.

## Command contract

At bootstrap, only `--help` and `--version` worked. The table remains the
design map; the development tree now implements `create`, `init`, `add`,
`remove`, `sync`, `run`, `python`, `tools`, and `doctor`. Other rows remain
planned. Command flags and limitations are documented in their guides.

| Command | Pyfaro responsibility | Delegated capability |
| --- | --- | --- |
| `create` | Local library and CLI templates with project metadata | None; uv is not invoked |
| `init` | Safe opt-in configuration of an existing project | uv metadata and locking |
| `add`, `remove` | Consistent dependency UX | uv dependency editing |
| `sync` | Locked/reproducible workflow | uv environment sync |
| `run` | Context and argument passthrough | uv execution |
| `python` | Interpreter command selection | uv Python version management |
| `tools` | Tool discovery and diagnostics | Executable/version probes |
| `dev` | Lifecycle, logs, preset selection | Project dev server |
| `lint`, `fmt` | Paths, flags, diagnostics | Ruff |
| `type` | Environment and invocation | ty |
| `check` | Non-writing aggregate quality gate | Ruff and ty |
| `test` | Environment, passthrough, status | pytest |
| `build` | Invocation and artifact diagnostics | uv build |
| `doctor` | Project and environment diagnostics | Tool probes and Python metadata |

`check` must not write files unless `--fix` is explicit. Direct `fmt` is an
intentional formatting operation; `fmt --check` is non-writing. Alternate
typing backends such as Pyright require a demonstrated need and a later ADR;
there is no public plugin API in the foundation.

## Module boundaries

- `main`: process entry point and eventual final error/status reporting.
- `cli`: Clap argument types and command dispatch boundary.
- Future `config`: typed settings and validation, preserving other tools.
- Future `core`: project discovery and cross-platform subprocess lifecycle.
- Future `adapters`: tool-specific arguments and version compatibility.

The bootstrap adds only modules with real behavior. Do not introduce generic
adapter traits or an async runtime until concrete requirements justify them.
Use typed errors at reusable boundaries and `Result` for recoverable failures;
render useful context at the CLI without exposing tokens or environment dumps.

## Exit codes and cancellation

- Help and version: 0. Invalid CLI usage: 2 (Clap conventions).
- Planned Pyfaro orchestration/configuration failure: 1 with an actionable
  stderr diagnostic. Distinguish it in text from delegated failures.
- A single delegated command forwards its numeric exit code, including pytest's
  distinct statuses. Aggregate `check` returns 1 if any check fails, otherwise 0.
- Planned user cancellation: 130 after forwarding cancellation and reaping
  children. Abnormal child termination without a numeric status: 1 with a
  diagnostic. Implement and test Unix signals and Windows process cleanup
  before claiming cross-platform cancellation support (F05).

The full stable error contract remains backlog B05. Use `Command` with explicit
arguments and per-child cwd, never an implicit shell. Preserve `OsString`
arguments and cross-platform paths. Stream output; cancellation must stop and
reap owned children without orphaning them. Shell tasks need explicit opt-in
and separate design work (D06).

## Versions, tool discovery, and reproducibility

The Rust binary uses a committed Cargo lockfile and pinned development
toolchain. Dependency updates must remain compatible with the declared MSRV
or explicitly update it with CI evidence.

External tools are not bundled or installed by the bootstrap. The planned
discovery order respects an explicitly configured path, the project environment,
then PATH, without surprise installation. Missing or unsupported versions must
produce actionable diagnostics. Pin known-working tool versions in integration
fixtures and generated project dev groups, preserve uv's native locked modes,
and document tested compatibility ranges as adapters land. No compatibility
with uv/Ruff/ty/pytest versions is claimed until those tests exist (F06, S01).

## Consequences

Pyfaro can focus on a cohesive experience and inherit upstream improvements.
It also depends on upstream command/version contracts, so adapters need focused
compatibility tests and explicit errors. Start with one binary crate and
observable CLI tests; grow disposable Python fixtures as integration lands.
Workspace scheduling and caching are deferred to v1.1.0, structured agent
interfaces to v1.2.0. Neither belongs in this bootstrap.
