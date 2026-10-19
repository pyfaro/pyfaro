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

The published experimental alphas do not yet have a versioned support policy.
Security fixes currently target `main`; a versioned support policy will
accompany the public beta.

## Trust boundaries

The current executable delegates initialization and environment operations to
uv, can run project programs, and probes external tool versions with the user's
privileges.
Treat untrusted repositories and their configuration as untrusted input.
Tool installation must be explicit; subprocesses must not use an implicit
shell. Pyfaro will not send telemetry by default. Any network use by delegated
tools must be documented separately from Pyfaro's own behavior.

Dependencies are checked by Dependabot and a scheduled RustSec audit. Report
upstream tool vulnerabilities to the relevant project as well when appropriate.
