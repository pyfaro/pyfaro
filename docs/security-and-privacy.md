# Security and privacy

This guide is the contract for what Pyfaro itself does with the network, what
it executes, and what a user trusts when installing it. The reader-facing
version is [Security and privacy](https://pyfaro.dev/concepts/security-and-privacy/)
on the site; keep the two in step. The vulnerability reporting process and the
supported-versions policy are in [SECURITY.md](../SECURITY.md).

## Pyfaro does not use the network

Pyfaro sends no telemetry, crash reports, usage statistics or update checks,
and has no setting that enables them. It never updates itself or any tool. It
keeps no global state: it reads the project and writes the files a command
documents, plus temporary files it removes.

Verified in `0.5.0-beta.1`:

- `Cargo.lock` has no HTTP or TLS client, telemetry, error-reporting, git
  transport or self-update crate. `tests/privacy.rs` fails `cargo test` when a
  crate from its deny list appears. The list covers HTTP clients and servers
  (`reqwest`, `hyper`, `ureq`, `curl`, `isahc`, `attohttpc`, `surf`, `h2`, ...),
  TLS stacks (`rustls`, `native-tls`, `openssl`, ...), resolvers and sockets
  above `std` (`hickory-resolver`, `socket2`, `quinn`), `git2`, telemetry
  (`sentry`, `opentelemetry`, ...) and self-update (`self_update`,
  `axoupdater`, ...). Test-only crates are in the lock file too, so the list
  covers them. A name is on the list when its only use in a CLI like this one
  would be talking to a remote host, reporting, or replacing the binary.
- `tokio` is not on the list. It is a general runtime, and it reaches the lock
  file on Windows because `subc-jobobject` (Job Objects for process cleanup)
  enables its `process` feature. Names cannot show features, so the test pins
  `subc-jobobject` as the only crate allowed to depend on `tokio`.
- `tests/privacy.rs` also checks that `src/` never mentions `TcpStream`,
  `UdpSocket`, `UnixStream` or `ToSocketAddrs`. The one socket Pyfaro opens is
  the `TcpListener` that `pyfaro dev` binds to see whether a port is free.
- `--verbose` writes only to standard error. URL credentials are masked and
  environment values are never printed (see [diagnostics](diagnostics.md)).

If a change needs a network client in Pyfaro, it needs a design decision first:
update this guide, the site page, the deny list and the changelog together.

## Network use of delegated tools

Checked against uv 0.9.26 by pointing `UV_INDEX_URL` and
`UV_PYTHON_INSTALL_MIRROR` at a local server that logs requests, with and
without `--offline`.

| Command | Network access | Who |
| --- | --- | --- |
| `create` | None; never runs uv. | - |
| `init` preview | None. | - |
| `init --apply` (Linux) | Index reads for `uv lock --no-build`. | uv |
| `sync` | Index reads if the lock must change; package downloads not in the cache. | uv |
| `add`, `remove` | Index reads to resolve and lock; `--no-sync`, so nothing is installed. | uv |
| `run`, `dev` | `uv run` locks and syncs first: as `sync`. Then the program runs. | uv, project code |
| `python list` | None: the catalog is built into uv (no request, online or offline). | - |
| `python install` | Interpreter download, by default from the `python-build-standalone` releases on GitHub. | uv |
| `python pin` | None. | - |
| `lint`, `fmt`, `type`, `check`, `test` | None from Pyfaro; Ruff, ty and pytest are started directly. | the tool |
| `build` | None with `uv_build` (built by uv itself); other backends are installed from the index into an isolated build environment. | uv |
| `tools`, `doctor` | None; version probes, and `doctor` runs uv with `--offline`. | - |
| `completions` | None. | - |

`--locked` and `--frozen` decide whether uv may rewrite `uv.lock`; packages
missing from uv's cache are still downloaded.

Controls:

- `--offline` (`init`, `sync`, `add`, `remove`, `run`, `dev`, `python`) passes
  `--offline` to uv, including for `python install`. `UV_OFFLINE=1` has the same
  effect because uv reads it.
- Pyfaro passes `--no-python-downloads` to every uv call except `python install`
  and `python list`. A flag beats the environment, so
  `UV_PYTHON_DOWNLOADS=automatic` cannot make `sync`, `run`, `dev` or `add`
  download an interpreter. `UV_PYTHON_DOWNLOADS=never` also blocks
  `python install`.
- Hosts are chosen by uv's own settings (`UV_INDEX_URL`, `[tool.uv]`,
  `UV_PYTHON_INSTALL_MIRROR`); Pyfaro does not rewrite them.

The install scripts talk to GitHub only: `api.github.com` for the newest
release, and the release download hosts. They accept `PYFARO_DOWNLOAD_BASE`
(`https://`, or a local `file://` directory) for tests.

## Third-party executables

| Program | Origin |
| --- | --- |
| uv | Installed by the user; found on `PATH` or `--uv`. Pyfaro probes `--version` and never installs or updates it. |
| Ruff, ty, pytest | Packages in the project environment, resolved by uv from the configured index. Templates pin `ruff==0.16.10`, `ty==0.0.85`, `pytest==9.1.1`; `uv.lock` records hashes. Lookup order: project environment, `VIRTUAL_ENV`, `PATH`. |
| Python | The user's interpreter, or one from `pyfaro python install`. |
| Template dependencies | `create fastapi`: `fastapi`, `uvicorn[standard]` and a pinned test client. `uv_build` is bundled with uv (building the project made no index request). |
| Project code | Tasks, `[tool.pyfaro.dev]`, `run`, `dev`, `test`. |

`tools` and `doctor` execute the project environment's `ruff`, `ty` and
`pytest` with `--version`; `doctor` runs the environment's Python with `-I -B`
in a scratch directory to read package metadata.

An untrusted repository is untrusted code, as with `uv run`: `sync` can build
source distributions, uv reads the project's index settings, tasks and the dev
process are the project's commands, and a `.venv` inside the repository is
searched first for tools. Pyfaro does not sandbox.

## Updates

No automatic update of Pyfaro, uv, tools or Python. Upgrades are explicit per
install method (install script rerun, `cargo install --force`, new archive;
`uv self update` for uv). Tool versions change when the user edits the pins and
runs `pyfaro sync`.

## Release trust

- `SHA256SUMS` lists every release asset; the install scripts verify it and
  refuse to install otherwise. It detects corruption and bad mirrors, not a
  compromised release.
- From `0.5.0-beta.1`, every archive and the `.crate` has a GitHub build
  provenance attestation (`gh attestation verify <file> --repo pyfaro/pyfaro`);
  earlier releases have none.
- Builds are reproducible and compared before upload; see
  [releasing](releasing.md).
- crates.io publishing uses trusted publishing with no long-lived token, after
  maintainer approval of the `crates-io` environment.
- Archives are not code-signed or notarized. On macOS, Gatekeeper only checks
  files that carry the `com.apple.quarantine` attribute, which browsers set
  and `curl` does not: the install script, `cargo install` and the PyPI wheels
  are not affected, a browser download needs `xattr -d com.apple.quarantine
  pyfaro` once. Windows SmartScreen may warn.
