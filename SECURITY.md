# Security policy

## Reporting a vulnerability

Please report security vulnerabilities privately using
[GitHub's private vulnerability reporting form](https://github.com/pyfaro/pyfaro/security/advisories/new).
You can also reach it through **Security → Report a vulnerability**.
Do not post exploit details, credentials, or sensitive data in public issues
or Discussions.

Include affected commit/version, platform, reproduction steps, impact, and
any suggested mitigation. Redact secrets and personal data. Maintainers will
coordinate investigation, a fix, and disclosure through the private advisory.
There is no guaranteed response SLA at this early stage. If the private form
is temporarily unavailable, use GitHub's private abuse-reporting channel for
exposed credentials or platform abuse; for a project vulnerability, wait for
private reporting to be restored rather than disclosing details publicly.

## Supported versions

During the public beta only the latest published version is supported.
Security fixes land on `main` first and ship in the next release; older
alphas and betas are not patched, so upgrade to the latest version (see the
[installation guide](https://pyfaro.dev/getting-started/installation/)).
Fixes are not backported.

## Trust boundaries

The full picture, with the network use of each command, is in
[Security and privacy](https://pyfaro.dev/concepts/security-and-privacy/)
([source](docs/security-and-privacy.md)). In short:

- **Pyfaro makes no network connections and sends no telemetry**, and never
  updates itself or other tools. A test fails the build if a network client
  enters the dependency graph. Network access comes from the tools it
  delegates to, mainly uv, and `--offline` disables it.
- **Pyfaro runs programs with your privileges and no sandbox**: uv (installed
  by you), Ruff, ty and pytest from the project environment, and the project's
  own tasks, dev process and code. Treat untrusted repositories and their
  configuration as untrusted input, as with `uv run`. Subprocesses never use an
  implicit shell.
- **Installation is explicit.** Pyfaro never installs or updates uv, Ruff, ty,
  pytest or Python on its own; an interpreter is downloaded only by
  `pyfaro python install`. The install scripts never use `sudo` or edit
  profiles.
- **Releases** have `SHA256SUMS`, and from `0.5.0-beta.1` build provenance
  attestations and reproducible builds. Binaries are not code-signed.

Dependencies are checked by Dependabot and a scheduled RustSec audit. Report
upstream tool vulnerabilities to the relevant project as well when appropriate.
