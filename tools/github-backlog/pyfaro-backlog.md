# Pyfaro — GitHub Issues Backlog

**65 issues** · **8 milestones** · **25 labels**

> Versions are planning milestones. `v0.3.0` is the minimum public MVP; `v0.5.0` is the proposed beta.

## Labels

Every issue has exactly one `type:*`, one `area:*` and one `priority:*` label.


## v0.1.0 — Foundation (9 issues)

Architecture, socle Rust, configuration et qualité du dépôt.

### F01 — [core] Define Pyfaro scope, command contract and architecture ADR

**Labels:** `type:research`, `area:core`, `priority:P0`  
**Dependencies:** None

Document what Pyfaro owns versus delegates (uv, Ruff, ty, pytest), compatibility rules and command naming.

**Acceptance criteria**

- [ ] ADR merged with CLI command matrix and exit-code semantics
- [ ] Explicitly forbid a second dependency resolver or lockfile
- [ ] Decision on external tool management and reproducibility

### F02 — [cli] Bootstrap the Rust CLI workspace

**Labels:** `type:feature`, `area:cli`, `priority:P0`  
**Dependencies:** `F01`

Create a buildable Rust CLI with modular subcommands, global flags and useful help.

**Acceptance criteria**

- [ ] `pyfaro --help` and `pyfaro --version` work
- [ ] Unknown commands return non-zero with a helpful message
- [ ] Release and debug builds succeed on Linux, macOS and Windows

### F03 — [config] Parse and validate [tool.pyfaro] in pyproject.toml

**Labels:** `type:feature`, `area:config`, `priority:P0`  
**Dependencies:** `F02`

Define typed configuration and meaningful errors for invalid user settings.

**Acceptance criteria**

- [ ] Supports default values, path resolution and schema validation
- [ ] Respects existing [tool.ruff], [tool.ty] and other tool sections
- [ ] No new standalone project metadata file required

### F04 — [core] Detect project roots and execution context

**Labels:** `type:feature`, `area:core`, `priority:P0`  
**Dependencies:** `F02`

Find the closest valid Python project without changing the current directory.

**Acceptance criteria**

- [ ] Walks ancestors to find pyproject.toml
- [ ] `--project <path>` overrides discovery
- [ ] Handles nested directories and symlinks deterministically

### F05 — [core] Build a cross-platform subprocess runner

**Labels:** `type:feature`, `area:core`, `priority:P0`  
**Dependencies:** `F02`

Implement safe process execution to delegate to external tools.

**Acceptance criteria**

- [ ] Preserves exit codes and forwards environment appropriately
- [ ] Supports streaming stdout/stderr and Ctrl-C cancellation
- [ ] Never invokes a shell implicitly for tool adapters

### F06 — [core] Discover uv and other tool versions

**Labels:** `type:feature`, `area:core`, `priority:P0`  
**Dependencies:** `F05`

Resolve uv, ruff, ty and pytest paths and report missing or incompatible tools.

**Acceptance criteria**

- [ ] Discovery respects virtual environments and PATH
- [ ] Version information is reported consistently
- [ ] Missing tools produce actionable errors, without unexpected installation

### F07 — [ci] Set up Rust checks and multi-OS CI

**Labels:** `type:chore`, `area:ci`, `priority:P0`  
**Dependencies:** `F02`

Protect trunk with formatting, Clippy, unit tests and smoke builds.

**Acceptance criteria**

- [ ] CI matrix covers Linux, macOS and Windows
- [ ] Rustfmt, Clippy and tests gate merging
- [ ] No credentials committed to repository

### F08 — [docs] Add project governance and issue templates

**Labels:** `type:docs`, `area:docs`, `priority:P1`  
**Dependencies:** None

Make the repository contributor-ready.

**Acceptance criteria**

- [ ] README explains goals and non-goals
- [ ] License, CONTRIBUTING, CODE_OF_CONDUCT and SECURITY files provided
- [ ] Bug report, feature request and pull-request templates available

### F09 — [testing] Create fixture-based CLI integration test harness

**Labels:** `type:testing`, `area:testing`, `priority:P0`  
**Dependencies:** `F05`, `F07`

Introduce disposable Python project fixtures and subprocess assertions.

**Acceptance criteria**

- [ ] Tests run without polluting user environment
- [ ] Fixtures cover success and failure output and exit codes
- [ ] CI executes smoke fixtures across supported OSes


## v0.2.0 — Project Management (9 issues)

Création de projets et gestion du runtime et des dépendances via uv.

### P01 — [uv] Implement the uv process adapter

**Labels:** `type:feature`, `area:uv`, `priority:P0`  
**Dependencies:** `F05`, `F06`

Delegate package and environment operations to uv with version-aware command construction.

**Acceptance criteria**

- [ ] uv output and exit codes propagate correctly
- [ ] Argument passing is safe on all OSes
- [ ] Integration tests use a pinned uv version

### P02 — [templates] Implement `pyfaro create` for libraries

**Labels:** `type:feature`, `area:templates`, `priority:P0`  
**Dependencies:** `P01`, `F03`

Generate a valid src-layout library project with Pyfaro defaults.

**Acceptance criteria**

- [ ] Creates pyproject.toml, src, tests and README
- [ ] Accepts name, target directory and Python version
- [ ] Generated project can sync, test and build

### P03 — [templates] Implement `pyfaro create` for CLIs

**Labels:** `type:feature`, `area:templates`, `priority:P1`  
**Dependencies:** `P02`

Generate an installable Python command-line project.

**Acceptance criteria**

- [ ] Produces valid console_scripts entry point
- [ ] Supports package and command naming
- [ ] Template syncs, runs smoke tests, and builds with the current Pyfaro workflows; aggregate quality checks are covered by Q07

### P04 — [config] Implement `pyfaro init` for existing projects

**Labels:** `type:feature`, `area:config`, `priority:P0`  
**Dependencies:** `F03`, `P01`

Opt in existing pyproject.toml projects without destructive changes.

**Acceptance criteria**

- [ ] Shows proposed modifications before applying
- [ ] Preserves third-party tool configuration and dependencies
- [ ] A second run is idempotent

### P05 — [uv] Implement `pyfaro add` and `pyfaro remove`

**Labels:** `type:feature`, `area:uv`, `priority:P0`  
**Dependencies:** `P01`

Expose dependency changes through uv, including groups and development dependencies.

**Acceptance criteria**

- [ ] Supports dependency specifiers and --dev
- [ ] Updates pyproject.toml and uv.lock via uv
- [ ] Returns useful solver errors

### P06 — [uv] Implement `pyfaro sync` and reproducible mode

**Labels:** `type:feature`, `area:uv`, `priority:P0`  
**Dependencies:** `P01`

Synchronize project dependencies and support frozen/locked workflows.

**Acceptance criteria**

- [ ] Default sync is delegated to uv
- [ ] `--locked` refuses stale lock state
- [ ] Does not maintain an independent lockfile

### P07 — [cli] Implement `pyfaro run` with argument passthrough

**Labels:** `type:feature`, `area:cli`, `priority:P0`  
**Dependencies:** `P01`, `F05`

Run scripts and commands in the project environment.

**Acceptance criteria**

- [ ] Preserves all arguments after `--`
- [ ] Executes inside the project environment
- [ ] Propagates command failures and exit codes

### P08 — [uv] Expose Python version management via uv

**Labels:** `type:feature`, `area:uv`, `priority:P1`  
**Dependencies:** `P01`

Provide thin wrappers for listing, installing and pinning interpreters.

**Acceptance criteria**

- [ ] `pyfaro python list|install|pin` delegates to uv
- [ ] Obeys requires-python and .python-version
- [ ] No custom Python installer or runtime cache

### P09 — [cli] Implement `pyfaro doctor` environment diagnostics

**Labels:** `type:feature`, `area:cli`, `priority:P1`  
**Dependencies:** `F03`, `F06`, `P01`

Explain problems with interpreters, tools, project layout and configuration.

**Acceptance criteria**

- [ ] Reports Python, uv and relevant tool versions
- [ ] Identifies conflicting configuration and missing dependencies
- [ ] Exits non-zero for blocking issues


## v0.3.0 — Code Quality (8 issues)

Lint, format, typage et tests avec une interface homogène — MVP.

### Q01 — [quality] Implement `pyfaro lint` with Ruff

**Labels:** `type:feature`, `area:quality`, `priority:P0`  
**Dependencies:** `P07`, `F03`

Lint configured project files by invoking Ruff.

**Acceptance criteria**

- [ ] Uses project Ruff configuration unchanged
- [ ] Supports target paths and normal exit codes
- [ ] Clearly reports when Ruff is unavailable

### Q02 — [quality] Implement `pyfaro fmt` with Ruff

**Labels:** `type:feature`, `area:quality`, `priority:P0`  
**Dependencies:** `Q01`

Expose Ruff formatting and non-mutating verification mode.

**Acceptance criteria**

- [ ] `pyfaro fmt` writes format changes
- [ ] `pyfaro fmt --check` never modifies files
- [ ] Uses project exclusions and Ruff settings

### Q03 — [quality] Implement `pyfaro type` with ty

**Labels:** `type:feature`, `area:quality`, `priority:P0`  
**Dependencies:** `P07`, `F03`

Run ty check in the resolved project environment.

**Acceptance criteria**

- [ ] Resolves project dependencies for type analysis
- [ ] Passes explicit paths and preserves ty diagnostics
- [ ] Documents limitations and supported ty version

### Q04 — [quality] Implement non-destructive `pyfaro check`

**Labels:** `type:feature`, `area:quality`, `priority:P0`  
**Dependencies:** `Q01`, `Q02`, `Q03`

Run formatting verification, lint and type checking as one quality gate.

**Acceptance criteria**

- [ ] Runs Ruff format --check, Ruff lint and ty check
- [ ] Does not modify source files by default
- [ ] Aggregate non-zero exit on any failure; readable summary

### Q05 — [quality] Add explicit `pyfaro check --fix`

**Labels:** `type:enhancement`, `area:quality`, `priority:P1`  
**Dependencies:** `Q04`

Apply supported safe formatter and linter fixes on request.

**Acceptance criteria**

- [ ] Never fixes source files without --fix
- [ ] Explains which checks can automatically fix issues
- [ ] Reruns verification and reports remaining issues

### Q06 — [testing] Implement `pyfaro test` with pytest

**Labels:** `type:feature`, `area:testing`, `priority:P0`  
**Dependencies:** `P07`

Run pytest through the project environment.

**Acceptance criteria**

- [ ] `pyfaro test` invokes pytest with correct cwd
- [ ] Forwards filters, marker arguments and test paths
- [ ] Returns pytest exit code and works in CI

### Q07 — [templates] Ship pinned quality and test defaults

**Labels:** `type:feature`, `area:templates`, `priority:P0`  
**Dependencies:** `P02`, `Q04`, `Q06`

New projects opt in to known-working Ruff, ty and pytest versions.

**Acceptance criteria**

- [ ] Generated templates include dev dependency group and lockfile instructions
- [ ] Tool configs are owned by their standard pyproject sections
- [ ] `pyfaro check` and `pyfaro test` pass immediately

### Q08 — [testing] Test quality commands on real fixtures

**Labels:** `type:testing`, `area:testing`, `priority:P0`  
**Dependencies:** `Q04`, `Q05`, `Q06`

Cover success, failures, exclusions and no-write-by-default.

**Acceptance criteria**

- [ ] Fixtures include library and CLI projects
- [ ] Tests assert check never writes files and --fix is explicit
- [ ] Pyfaro returns consistent codes for lint/type/test failures


## v0.4.0 — Developer Experience (8 issues)

Mode développement, presets et tâches simples.

### D01 — [templates] Add FastAPI project preset

**Labels:** `type:feature`, `area:templates`, `priority:P0`  
**Dependencies:** `P02`, `Q07`

Generate a small functional FastAPI application with tests.

**Acceptance criteria**

- [ ] Preset includes application entry point and smoke test
- [ ] Project works with create, dev, check and test
- [ ] Dependencies are managed through uv

### D02 — [dev] Implement process supervisor for `pyfaro dev`

**Labels:** `type:feature`, `area:dev`, `priority:P0`  
**Dependencies:** `F05`, `P07`

Run a long-lived development process with clean lifecycle management.

**Acceptance criteria**

- [ ] Ctrl-C stops child processes without orphaning
- [ ] Correct environment and working directory
- [ ] Exit errors surface clearly

### D03 — [dev] Add FastAPI dev runner and reload

**Labels:** `type:feature`, `area:dev`, `priority:P0`  
**Dependencies:** `D01`, `D02`

Start Uvicorn with an appropriate reload configuration for FastAPI presets.

**Acceptance criteria**

- [ ] `pyfaro dev` starts the template app
- [ ] Edits trigger reload and syntax errors are visible
- [ ] No second bespoke filesystem watcher for FastAPI

### D04 — [config] Configure dev entry point and arguments

**Labels:** `type:feature`, `area:config`, `priority:P1`  
**Dependencies:** `D02`, `F03`

Add [tool.pyfaro.dev] settings for project-specific development commands.

**Acceptance criteria**

- [ ] Supports entry point, arguments and reload switch
- [ ] Explicit setting takes precedence over preset
- [ ] Invalid values are detected before launch

### D06 — [tasks] Specify safe task invocation semantics

**Labels:** `type:research`, `area:tasks`, `priority:P0`  
**Dependencies:** `P07`

Define shell/no-shell behavior, quoting, cancellation and environment handling.

**Acceptance criteria**

- [ ] Documented resolution of tasks vs executable programs
- [ ] Explicit opt-in for shell constructs and no implicit shell injection
- [ ] Cross-platform escaping semantics tested

### D05 — [tasks] Implement named custom tasks

**Labels:** `type:feature`, `area:tasks`, `priority:P1`  
**Dependencies:** `P07`, `F03`, `D06`

Support named commands configured under [tool.pyfaro.tasks].

**Acceptance criteria**

- [ ] `pyfaro run <task>` executes a named task
- [ ] Task and program names cannot collide silently
- [ ] Explicit cwd and environment fields are supported

### D07 — [dev] Improve live development logs and errors

**Labels:** `type:enhancement`, `area:dev`, `priority:P1`  
**Dependencies:** `D02`

Provide legible prefixed logs, signal handling and startup diagnostics.

**Acceptance criteria**

- [ ] stdout/stderr remains readable and Ctrl-C reliable
- [ ] Failure to bind a port produces useful guidance
- [ ] Verbose mode reveals invoked command

### D08 — [docs] Write end-to-end FastAPI getting-started guide

**Labels:** `type:docs`, `area:docs`, `priority:P1`  
**Dependencies:** `D01`, `D03`

Document the workflow from install to a running app and passing CI.

**Acceptance criteria**

- [ ] Tutorial demonstrates create, add, dev, check, test and build
- [ ] Examples work on at least Linux and Windows
- [ ] Known prerequisites and tool versions documented


## v0.5.0 — Public Beta (8 issues)

Build, installations multiplateformes et première distribution publique.

### B01 — [uv] Implement `pyfaro build` using uv build

**Labels:** `type:feature`, `area:uv`, `priority:P0`  
**Dependencies:** `P01`, `P02`

Build Python wheels and sdists without reimplementing packaging.

**Acceptance criteria**

- [ ] Supports wheel and sdist artifacts
- [ ] Preserves PEP 517/518 build-backend configuration
- [ ] Validates generated artifacts in CI

### B02 — [distribution] Build standalone Pyfaro executables

**Labels:** `type:feature`, `area:distribution`, `priority:P0`  
**Dependencies:** `F02`, `F07`

Produce reproducible release artifacts for supported platforms.

**Acceptance criteria**

- [ ] Artifacts for Linux x86_64, macOS x86_64/aarch64, and Windows x86_64
- [ ] Artifacts contain CLI and display version
- [ ] Checksums published alongside archives

### B03 — [distribution] Provide secure installation and upgrade path

**Labels:** `type:feature`, `area:distribution`, `priority:P0`  
**Dependencies:** `B02`, `P09`

Ship installation instructions/scripts and a clear path to install uv prerequisites.

**Acceptance criteria**

- [ ] Documented installation on all supported OSes
- [ ] Verify downloaded binaries/checksums; no silent unsafe installers
- [ ] `pyfaro doctor` explains missing uv and offers actionable steps

### B04 — [cli] Add shell completion and friendly help

**Labels:** `type:enhancement`, `area:cli`, `priority:P1`  
**Dependencies:** `F02`

Improve interactive CLI discoverability.

**Acceptance criteria**

- [ ] Supports Bash, Zsh and Fish completions
- [ ] Top-level help includes realistic workflow examples
- [ ] CLI flags documented consistently

### B05 — [core] Standardize error codes and verbose diagnostics

**Labels:** `type:enhancement`, `area:core`, `priority:P0`  
**Dependencies:** `F05`, `P09`

Expose consistent failures and command invocation traces for troubleshooting.

**Acceptance criteria**

- [ ] Document stable exit codes (including tool forwarding rules)
- [ ] `--verbose` displays tool versions and executed commands
- [ ] Errors avoid leaking tokens or sensitive environment values

### B06 — [ci] Set up GitHub Releases and crates.io publication

**Labels:** `type:chore`, `area:ci`, `priority:P0`  
**Dependencies:** `B02`

Automate signed/tagged public pre-releases with documented ownership.

**Acceptance criteria**

- [ ] Tag-to-release pipeline publishes verified artifacts
- [ ] crate version and CLI version match release tag
- [ ] Publish credentials minimized; release notes generated

### B07 — [docs] Publish CLI and configuration reference

**Labels:** `type:docs`, `area:docs`, `priority:P0`  
**Dependencies:** `B01`, `B03`

Document all public commands and configuration defaults.

**Acceptance criteria**

- [ ] Each command has usage, flags and exit codes
- [ ] [tool.pyfaro] configuration examples are copyable
- [ ] Installation and troubleshooting documented

### B08 — [security] Document data/privacy and install trust model

**Labels:** `type:docs`, `area:security`, `priority:P1`  
**Dependencies:** `B03`

Set default-no-telemetry stance and identify download and execution trust boundaries.

**Acceptance criteria**

- [ ] No telemetry or outbound reporting from Pyfaro by default
- [ ] Document uv/network accesses distinctly
- [ ] Explicit disclosure of third-party binaries and updates


## v1.0.0 — Stable Release (8 issues)

Compatibilité, sécurité, performance, documentation et engagements de stabilité.

### S01 — [core] Create supported-version compatibility matrix

**Labels:** `type:testing`, `area:core`, `priority:P0`  
**Dependencies:** `Q08`, `B06`

Test supported Python/uv/Ruff/ty/pytest combinations and platforms.

**Acceptance criteria**

- [ ] Maintained version support policy
- [ ] CI matrix includes defined Python versions and OSes
- [ ] Failures of pinned integration versions are detected

### S02 — [config] Validate existing-project interoperability

**Labels:** `type:testing`, `area:config`, `priority:P0`  
**Dependencies:** `P04`, `Q04`

Ensure Pyfaro does not rewrite or break conventional uv-based Python projects.

**Acceptance criteria**

- [ ] Existing pyproject.toml tool sections remain untouched
- [ ] Sync/check/test work with common src layouts
- [ ] No unnecessary changes to uv.lock or dependencies

### S03 — [security] Threat-model subprocesses and release supply chain

**Labels:** `type:chore`, `area:security`, `priority:P0`  
**Dependencies:** `D06`, `B06`

Harden process execution, binary supply and CI permissions.

**Acceptance criteria**

- [ ] Argument injection, task shell opt-in and path traversal covered by tests
- [ ] Checksums or attestations for release assets
- [ ] Least-privilege permissions for release automation

### S04 — [core] Define backwards compatibility and deprecation policy

**Labels:** `type:docs`, `area:core`, `priority:P0`  
**Dependencies:** `B05`, `B07`

Freeze the public v1 command, configuration and output contracts.

**Acceptance criteria**

- [ ] Compatibility guarantees for flags and [tool.pyfaro] documented
- [ ] Breaking changes and deprecation mechanism defined
- [ ] External tool versions and lock behavior spelled out

### S05 — [ci] Add end-to-end smoke matrix on clean machines

**Labels:** `type:testing`, `area:ci`, `priority:P0`  
**Dependencies:** `B03`, `B06`, `D03`

Verify first install, create, run, check, test and build without developer-specific setup.

**Acceptance criteria**

- [ ] Smoke tests on clean Linux/macOS/Windows runners
- [ ] New user flow executes from fresh checkout
- [ ] No hidden PATH or local environment dependency

### S06 — [core] Establish baseline performance benchmarks

**Labels:** `type:testing`, `area:core`, `priority:P1`  
**Dependencies:** `Q04`, `B01`

Measure CLI startup and representative workflows without inventing performance claims.

**Acceptance criteria**

- [ ] Benchmarks cover startup, check, test and create
- [ ] Results compared against direct underlying tools
- [ ] Benchmarks published and regressions investigated

### S07 — [docs] Publish migration, troubleshooting and examples

**Labels:** `type:docs`, `area:docs`, `priority:P1`  
**Dependencies:** `S02`, `D08`

Prepare complete public documentation for stable release.

**Acceptance criteria**

- [ ] Migration guide for uv + Ruff + pytest projects
- [ ] Examples for library, CLI and FastAPI workflows
- [ ] Common failure modes and limitations documented

### S08 — [distribution] Ship Pyfaro 1.0 with changelog and support policy

**Labels:** `type:chore`, `area:distribution`, `priority:P0`  
**Dependencies:** `S01`, `S03`, `S04`, `S05`, `S07`

Make stable release with verified install commands and stable docs.

**Acceptance criteria**

- [ ] No unresolved milestone-blocking P0 defects
- [ ] Versioned release notes and reproducible artifacts
- [ ] Stable download instructions and support matrix published


## v1.1.0 — Workspaces & Tasks (8 issues)

Workspaces, graphe de tâches, parallélisme et cache.

### W01 — [workspaces] Discover uv workspace members

**Labels:** `type:feature`, `area:workspaces`, `priority:P0`  
**Dependencies:** `S08`

Use native uv workspaces rather than creating an independent package graph.

**Acceptance criteria**

- [ ] Finds root and members from uv-compatible configuration
- [ ] Determines active member from current directory
- [ ] Clear errors for broken workspace definitions

### W02 — [workspaces] Add package-scoped command selection

**Labels:** `type:feature`, `area:workspaces`, `priority:P0`  
**Dependencies:** `W01`

Run check/test/tasks against selected workspace packages.

**Acceptance criteria**

- [ ] Supports --package and --all where meaningful
- [ ] Stable member selection and ordered output
- [ ] Workspace filters do not alter uv workspace metadata

### W03 — [tasks] Define dependency-aware task DAG

**Labels:** `type:feature`, `area:tasks`, `priority:P0`  
**Dependencies:** `D05`

Allow declared task dependencies and reject cycles.

**Acceptance criteria**

- [ ] Supports depends-on links in task configuration
- [ ] Cycle and missing-task errors name offending chain
- [ ] Does not execute a task more than once per graph run

### W04 — [tasks] Execute independent tasks in parallel

**Labels:** `type:feature`, `area:tasks`, `priority:P1`  
**Dependencies:** `W03`

Schedule task DAG with bounded concurrency.

**Acceptance criteria**

- [ ] Configurable concurrency; deterministic failure summary
- [ ] Cancel remaining work on Ctrl-C
- [ ] No accidental interleaving that obscures task identity

### W05 — [tasks] Add opt-in local content-addressed task cache

**Labels:** `type:feature`, `area:tasks`, `priority:P1`  
**Dependencies:** `W03`

Avoid repeating deterministic tasks when declared inputs are unchanged.

**Acceptance criteria**

- [ ] Cache key includes inputs, config and toolchain versions
- [ ] Tasks declare outputs; side-effect tasks are uncached by default
- [ ] Manual --no-cache switch and explainable hits/misses

### W06 — [tasks] Implement cache safety and invalidation tests

**Labels:** `type:testing`, `area:tasks`, `priority:P0`  
**Dependencies:** `W05`

Verify stale artifacts, environmental changes and deletion do not cause false hits.

**Acceptance criteria**

- [ ] Changed inputs and tool versions invalidate cache
- [ ] Missing or altered outputs cause reruns
- [ ] Cache corruption falls back to normal execution

### W07 — [tasks] Add `pyfaro tasks list|graph` inspector

**Labels:** `type:enhancement`, `area:tasks`, `priority:P1`  
**Dependencies:** `W03`

Help developers understand declared tasks and task ordering.

**Acceptance criteria**

- [ ] List displays task summary and source config
- [ ] Graph displays dependencies and detects cycles
- [ ] Text output works without terminal UI

### W08 — [ci] Demonstrate incremental monorepo checks in CI

**Labels:** `type:docs`, `area:ci`, `priority:P1`  
**Dependencies:** `W02`, `W05`

Document repeatable multi-package checks and caching.

**Acceptance criteria**

- [ ] Example workspace project with shared library and API
- [ ] CI example runs selective checks and tests
- [ ] Explains why uv caches and task caches are different


## v1.2.0 — Agent Integration (7 issues)

Diagnostics et workflows structurés pour agents de programmation.

### A01 — [agents] Specify a versioned JSON output contract

**Labels:** `type:research`, `area:agents`, `priority:P0`  
**Dependencies:** `S04`

Define machine-readable results for check, test, doctor and tasks.

**Acceptance criteria**

- [ ] Schema defines tool version, command, status and structured errors
- [ ] Schema has explicit version field and backwards-compatibility rules
- [ ] No parsing ANSI terminal output to produce JSON

### A02 — [agents] Implement `--output json` across diagnostics

**Labels:** `type:feature`, `area:agents`, `priority:P0`  
**Dependencies:** `A01`

Expose structured results consistently to tools and agents.

**Acceptance criteria**

- [ ] check/test/doctor support JSON where adapters expose data
- [ ] stdout valid JSON; human-friendly logs on stderr when needed
- [ ] Exit codes remain meaningful

### A03 — [agents] Normalize file diagnostics and source locations

**Labels:** `type:feature`, `area:agents`, `priority:P0`  
**Dependencies:** `A02`

Convert available Ruff/ty/pytest information into a shared diagnostic model.

**Acceptance criteria**

- [ ] Includes severity, message, tool and file/range when available
- [ ] Preserves upstream diagnostic details in an extension field
- [ ] Missing source ranges are represented explicitly, not invented

### A04 — [agents] Introduce changed-files quality checks

**Labels:** `type:feature`, `area:agents`, `priority:P1`  
**Dependencies:** `Q04`, `A02`

Check only affected paths when supported by underlying tools.

**Acceptance criteria**

- [ ] Supports explicit path selection and optional git-based change discovery
- [ ] Defines behavior for renames/deleted files
- [ ] Does not claim sound incremental typing if dependencies require full project checking

### A05 — [agents] Provide CI-friendly annotations and SARIF export

**Labels:** `type:enhancement`, `area:agents`, `priority:P1`  
**Dependencies:** `A03`

Make normalized diagnostics consumable by code scanning interfaces.

**Acceptance criteria**

- [ ] SARIF export documents supported fields
- [ ] GitHub annotations point to correct source locations
- [ ] Machine output tested against known fixtures

### A06 — [agents] Add agent-safe execution guidelines and examples

**Labels:** `type:docs`, `area:agents`, `priority:P1`  
**Dependencies:** `A02`, `A04`

Document non-interactive operation and low-risk workflows for AI coding agents.

**Acceptance criteria**

- [ ] Example AGENTS.md template and deterministic command sequence
- [ ] No auto-fix without explicit request
- [ ] Document which commands can write files or contact network

### A07 — [core] Draft RFC for future plugin/adapter API

**Labels:** `type:research`, `area:core`, `priority:P2`  
**Dependencies:** `S08`

Evaluate demand for alternate type checkers, test runners and framework adapters before committing to a public plugin API.

**Acceptance criteria**

- [ ] RFC proposes extension points and versioning approach
- [ ] Defines out-of-scope changes (no alternate lockfile/backend)
- [ ] No public plugin API shipped without proven use cases
