# Continuous integration

This page shows a GitHub Actions workflow that installs uv and Pyfaro, then
runs the same three commands you run locally: `sync --locked`, `check` and
`test`. If your project passes them on your machine, it passes in CI.

## The workflow

Save this as `.github/workflows/ci.yml`. It installs Pyfaro from a GitHub
Release archive and verifies the download against the release's `SHA256SUMS`.

=== "Release archive"

    ```yaml
    name: CI

    on:
      push:
        branches: [main]
      pull_request:

    permissions:
      contents: read

    jobs:
      quality:
        runs-on: ubuntu-latest
        env:
          PYFARO_VERSION: 0.3.0-alpha.1
        steps:
          - uses: actions/checkout@v7
            with:
              persist-credentials: false

          - name: Install uv
            uses: astral-sh/setup-uv@v7
            with:
              version: "0.9.26"

          - name: Install pyfaro
            run: |
              set -euo pipefail
              name="pyfaro-${PYFARO_VERSION}-x86_64-unknown-linux-gnu"
              url="https://github.com/pyfaro/pyfaro/releases/download/v${PYFARO_VERSION}"
              curl -fsSLO "${url}/${name}.tar.gz"
              curl -fsSLO "${url}/SHA256SUMS"
              grep " ${name}.tar.gz\$" SHA256SUMS | sha256sum --check -
              tar -xzf "${name}.tar.gz"
              mkdir -p "$HOME/.local/bin"
              install "${name}/pyfaro" "$HOME/.local/bin/pyfaro"
              echo "$HOME/.local/bin" >> "$GITHUB_PATH"
              rm -rf SHA256SUMS "${name}" "${name}.tar.gz"

          - name: Show tool versions
            run: |
              pyfaro --version
              uv --version

          - name: Sync the locked environment
            run: pyfaro sync --locked

          - name: Check formatting, lint and types
            run: pyfaro check

          - name: Run the tests
            run: pyfaro test
    ```

=== "cargo install"

    Use the same file, but replace the "Install pyfaro" step with a build from
    crates.io. GitHub-hosted runners ship a Rust toolchain recent enough for
    Pyfaro (1.90 or newer is required); compiling takes longer than
    downloading an archive.

    ```yaml
          - name: Install pyfaro
            run: cargo install pyfaro --version "${PYFARO_VERSION}" --locked
    ```

Both variants pass [actionlint](https://github.com/rhysd/actionlint). The
archive download, checksum verification, `sync --locked`, `check` and `test`
steps were run against a real project.

## What each step does

`astral-sh/setup-uv`
:   Installs uv. Pyfaro requires uv 0.9.26 or newer and never installs it
    itself; the example pins the tested version.

Install pyfaro
:   Downloads the archive, checks its SHA-256 against `SHA256SUMS`, and puts
    `pyfaro` on the `PATH`. Archives are published for Linux (x86_64,
    aarch64), macOS (arm64, x86_64) and Windows (x86_64, a `.zip`); change
    the target triple in `name` to match your runner.

`pyfaro sync --locked`
:   Creates `.venv` from `uv.lock` and fails if the lock file is missing or
    out of date. That is what you want in CI: a stale lock is a bug to fix,
    not to paper over. Commit `uv.lock`.

`pyfaro check`
:   Format check, Ruff lint and ty. Exits 1 if any step fails.

`pyfaro test`
:   Runs pytest in the project environment and returns pytest's exit code.

`check` and `test` do not sync implicitly, so `sync` must come first.

## Several Python versions

Run the job on a matrix and let uv pick the interpreter through `UV_PYTHON`:

```yaml
jobs:
  quality:
    runs-on: ubuntu-latest
    strategy:
      fail-fast: false
      matrix:
        python-version: ["3.11", "3.12", "3.13"]
    env:
      UV_PYTHON: ${{ matrix.python-version }}
```

uv downloads an interpreter if the runner lacks it. Keep every version inside
your `requires-python` range; see [Python versions](python-versions.md).

!!! tip "Pin actions by commit SHA"
    The example uses version tags for readability. For supply-chain safety,
    pin third-party actions to a full commit SHA.

!!! note "Pyfaro is alpha"
    Pin `PYFARO_VERSION` so a new alpha does not change your CI unexpectedly,
    and bump it deliberately.

## Next steps

- Reproduce a failure locally with the same commands: [The quality gate](quality-gate.md).
- Exit codes and common errors: [Troubleshooting](troubleshooting.md).
