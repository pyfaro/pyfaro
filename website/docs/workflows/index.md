---
description: "Step-by-step Pyfaro recipes: a new library, a new CLI, a FastAPI app, adopting a project, CI and troubleshooting."
---

# Workflows

Commands are the building blocks; workflows are the recipes. Each page here
combines several `pyfaro` commands into an end-to-end task, and every example
was run with a real uv and the real output is shown.

<div class="grid cards" markdown>

-   :material-bookshelf: **[A new library](new-library.md)**

    Create, sync, add a dependency, check, test and build a distributable
    package.

-   :material-console: **[A new CLI](new-cli.md)**

    Scaffold a command-line app, run it in the project environment and
    install it as a tool.

-   :material-api: **[A FastAPI app](fastapi.md)**

    From install to a running server and a passing CI workflow: create,
    add, dev, check, test, tasks and build.

-   :material-folder-swap: **[Adopt an existing project](adopt-existing-project.md)**

    Preview and apply `init`, then bring quality tools into a project you
    already have.

-   :material-shield-check: **[The quality gate](quality-gate.md)**

    Run `check` before every commit, let `--fix` tidy up, and wire it into a
    git hook.

-   :material-github: **[Continuous integration](continuous-integration.md)**

    A GitHub Actions workflow that installs uv and Pyfaro, then syncs,
    checks and tests.

-   :material-language-python: **[Python versions](python-versions.md)**

    List, install and pin interpreters and keep them consistent with
    `requires-python`.

-   :material-lifebuoy: **[Troubleshooting](troubleshooting.md)**

    Common errors from `doctor`, `tools` and friends, with their fixes and
    exit codes.

</div>

!!! note "Before you start"
    These workflows assume Pyfaro and uv are installed (uv 0.9.26 or newer).
    Pyfaro never installs uv or Python for you; see
    [Installation](../getting-started/installation.md).
