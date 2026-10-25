# Getting started

Pyfaro is a single command-line tool that puts one workflow on top of
[uv](https://docs.astral.sh/uv/), [Ruff](https://docs.astral.sh/ruff/),
[ty](https://docs.astral.sh/ty/) and [pytest](https://docs.pytest.org/).
It creates projects, manages dependencies and environments, and runs the
quality gate, without resolving packages or linting code itself.

!!! warning "Beta"
    The latest published version is `0.5.0-beta.1`, the first public beta.
    Interfaces may still change before 1.0. Only the commands listed under
    [Features](features.md) exist today.

<div class="grid cards" markdown>

-   :material-download: **[Installation](installation.md)**

    Install Pyfaro with the install script, Cargo or a prebuilt archive, and
    install uv.

-   :material-timer-outline: **[First steps](first-steps.md)**

    A five-minute tour: create, sync, add, run, check, test and doctor.

-   :material-api: **[A FastAPI app](../workflows/fastapi.md)**

    The end-to-end guide: create an API, run it with `dev`, test it and
    check it in CI.

-   :material-view-list: **[Features](features.md)**

    Every implemented command, with a link to its page.

-   :material-lightbulb-outline: **[Concepts](../concepts/index.md)**

    How Pyfaro delegates to the tools it wraps.

</div>

## What you need

- A supported platform: Linux, macOS or Windows (see
  [platform support](../concepts/platform-support.md)).
- [uv](https://docs.astral.sh/uv/) 0.9.26 or newer. Pyfaro never installs it.
- A Python interpreter that uv can find. Pyfaro never downloads one unless
  you ask with [`pyfaro python install`](../commands/python.md).

## Next steps

- [Install Pyfaro](installation.md).
- Follow the [first steps](first-steps.md).
- Browse the [commands](../commands/index.md) or the [workflows](../workflows/index.md).
