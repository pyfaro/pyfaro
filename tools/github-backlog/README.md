# Pyfaro — GitHub backlog kit

An actionable draft roadmap for the Pyfaro open-source Python toolchain.

- `pyfaro-backlog.md` — human-readable breakdown of every issue, acceptance criteria, and dependency IDs.
- `pyfaro-issues.json` — structured manifest (8 milestones, 25 labels, 65 issues).
- `pyfaro-issues.csv` — tabular view for filtering/reviewing the backlog.
- `import_github.py` — optional, **dry-run by default**, issue import using the GitHub CLI (`gh`).

## Import into GitHub

Prerequisites: GitHub CLI installed and authenticated (`gh auth login`), and a repository already created.

```bash
# Inspect without creating anything:
python import_github.py --repo pyfaro/pyfaro

# Import every official milestone and issue:
python import_github.py --repo pyfaro/pyfaro --apply

# Optional for other repositories: select only the MVP (26 issues).
python import_github.py --repo OWNER/REPO --through v0.3.0 --apply
```

**Caution:** `--apply` can create 65 real issues. Review the dry-run first.
Preserve the official manifest's scope and metadata in `pyfaro/pyfaro`.

## Maintained importer behavior

The JSON manifest is the authoritative backlog. Keep the supplied Markdown
and CSV snapshots for reference; do not silently change milestone or feature
scope. This repository imports **all 65 issues**; the partial-MVP example above
is optional for other repositories.

The maintained importer validates unique IDs/titles, label categories, and an
acyclic dependency graph before any API calls. Dry-run is offline and does not
mutate GitHub. `--apply` uses paginated API reads, matches stable backlog IDs
before exact titles, and fails on ambiguous or unmanaged title collisions.
It resumes after interruption without blindly repeating issue creation.

Existing managed label metadata and issue titles/labels/milestones are reconciled
to the manifest. Issue prose, checkbox state, comments, and open/closed state
are preserved. The importer only rewrites a recognizable generated dependency
section, retaining symbolic IDs alongside actual issue URLs, then creates
missing native GitHub `blocked_by` relations. Unexpected edited sections or
unavailable dependency APIs stop the import visibly; resolve the cause and
rerun. Never run two imports concurrently. Ambiguous failed writes are not
automatically retried: reread remote IDs on the next run.

After applying, fresh reads verify metadata, acceptance criteria, and native
dependency edges. `issue-map.json` records backlog IDs, issue numbers, and URLs.
No issue is automatically closed, and no releases or tags are created.

The committed [issue map](issue-map.json) links the official symbolic IDs to
the live tracker. The initial full import created 65 issues. An interrupted
dependency-linking pass was resumed by reusing all 65, with no duplicates.

```sh
# From this directory, validate the complete plan and run importer tests:
python import_github.py --repo pyfaro/pyfaro
python -m unittest discover -s . -p 'test_*.py'

# Apply/resume, then independently verify without mutations:
python import_github.py --repo pyfaro/pyfaro --apply
python import_github.py --repo pyfaro/pyfaro --verify
```

The native dependency API requires issue write permissions; verification needs
read access. GitHub rate/permission errors are surfaced rather than treated as
successful imports. Mutation calls are paced to respect secondary rate limits.

## Proposed scope boundaries

- `v0.3.0` is the first useful MVP: project creation, dependency commands, lint, fmt, type checking and tests.
- `v0.5.0` is the beta including install and build distribution.
- `v1.0.0` is for stability/security/compatibility rather than feature creep.
- The DAG/cache/workspaces and agent interfaces are deferred to `v1.1.0` and `v1.2.0`.

Python dependency resolution, runtime management and packaging stay delegated to **uv**. Code checks delegate to **Ruff** and **ty**. Testing delegates to **pytest**.
