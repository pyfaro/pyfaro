# Repository bootstrap record

## Initial state and decisions

The `pyfaro/pyfaro` repository did not exist at audit time. The authenticated
organization administrator created it as a public repository and cloned it
into a new local directory. There was no existing history or user work.
The initial bootstrap was committed directly to `main`; subsequent changes
use a setup branch and pull request.

Apache-2.0 was selected because no license existed. The Rust binary uses Clap,
a committed Cargo lockfile, and Rust 1.90.0 for development and CI. The only
implemented interface is help and version. Workflow commands remain planned.

## Backlog

The supplied JSON, CSV, and Markdown backlog snapshots are preserved. The
importer was inspected and extended for stable-ID resume, pagination, native
dependencies, conservative body updates, validation, and verification.
The full dry-run confirmed 25 labels, 8 milestones, and 65 issues.

The import created all 65 issues. A dependency pass was interrupted and resumed
with 0 new issues and 65 reused issues. Fresh API reads verified all titles,
summaries, acceptance criteria, labels, milestones, symbolic IDs, linked URLs,
and **105 native `blocked_by` relationships**. All issues remain open.
See [the issue map](../tools/github-backlog/issue-map.json).

The ten unused labels automatically added to the newly created repository
were removed after the authoritative labels were created. The final tracker
has exactly 25 manifest labels and 8 manifest milestones, with issue counts
9 / 9 / 8 / 8 / 8 / 8 / 8 / 7 respectively. No releases or tags were created.

## Verification evidence

- Initial [CI run](https://github.com/pyfaro/pyfaro/actions/runs/38694243685):
  all three OS jobs and the backlog validation job passed.
- Linux, macOS, and Windows each passed formatting, Clippy with warnings denied,
  3 CLI integration tests, debug/release builds, help, and version checks.
- The backlog job passed the offline dry-run and 6 importer tests.
- Initial [security run](https://github.com/pyfaro/pyfaro/actions/runs/38694243669):
  cargo-audit 0.22.2 passed against RustSec.
- Local checks passed using the installed Rust 1.99.0; hosted CI independently
  verified the pinned Rust 1.90.0 toolchain. Actionlint 1.7.12 accepted both
  workflows. The local RustSec audit passed as well.

## GitHub settings

- Public repository; `main` default branch; Issues and Discussions enabled.
- Squash merging enabled; merge commits and rebase merging disabled.
- Automatic deletion of merged PR head branches enabled.
- Private vulnerability reporting, Dependabot alerts/security updates, secret
  scanning, and secret-scanning push protection enabled and verified.
- Dependabot checks Cargo and GitHub Actions weekly.
- Workflow default permissions are read-only; workflows cannot approve PRs.
- Actions are pinned to immutable commit SHAs; checkout does not persist
  credentials. No publishing credentials or registry secrets were introduced.

After the checks existed and passed, branch protection was applied to `main`:

- Require PRs and resolved review conversations, with **0 required approvals**
  so a sole maintainer can merge.
- Require up-to-date branches and the GitHub Actions checks
  `Rust (ubuntu-latest)`, `Rust (macos-latest)`, `Rust (windows-latest)`,
  `Backlog manifest`, and `RustSec audit`.
- Enforce protections for administrators and require linear history.
- Disallow force pushes and branch deletion.

## Limitation and next step

The optional **Pyfaro Development Roadmap** Project could not be created:
the current OAuth token has `repo`, `workflow`, `read:org`, and `gist`, but no
`read:project`/`project` scope. GitHub rejected the project query with
`INSUFFICIENT_SCOPES`. No organization-wide permissions or billing were changed.
Milestones and linked issue dependencies provide the initial planning surface.

Start with [F01 / #1](https://github.com/pyfaro/pyfaro/issues/1): review and
finalize the command, compatibility, and exit-code contract against ADR 0001.
Review full acceptance criteria before closing any scaffold-related issue.
