# Future publishing workflow

Pyfaro is not released yet. B02 and B06 track distribution and publication.
`publish = false` in `Cargo.toml` prevents accidental crates.io publication.

Before the first release, maintainers should:

1. Verify ownership and availability of the crates.io name and decide the
   supported platform/tool-version matrix.
2. Finish the milestone acceptance criteria, review the lockfile and licenses,
   update the changelog, and align Cargo and CLI versions.
3. Build and test debug/release artifacts on supported platforms; run
   `cargo package --locked` and a publication dry-run after intentionally
   enabling publication in a reviewed PR.
4. Design a tag-triggered release workflow with least privileges, protected
   release approval, checksums, artifact provenance, and a documented rollback
   and credential-rotation procedure. Prefer trusted publishing when supported;
   otherwise scope a registry token narrowly to the release environment.
5. Publish only after explicit maintainer approval and verified checks. A tag
   must describe shipped code, not an uncompleted roadmap milestone.

No publication workflow, registry secret, release tag, or GitHub Release is
created during repository bootstrap.
