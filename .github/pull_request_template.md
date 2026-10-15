## Summary

Explain the change and why it is needed.

## Related issues

Reference the issue and backlog ID. Use closing keywords only when all
acceptance criteria are satisfied.

## Verification

List commands run and outcomes, including relevant platform evidence.

- [ ] `cargo fmt --all -- --check`
- [ ] `cargo clippy --locked --all-targets --all-features -- -D warnings`
- [ ] `cargo test --locked --all-targets`
- [ ] `cargo build --locked`
- [ ] Documentation reflects implemented behavior
