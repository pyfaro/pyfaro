//! End-to-end tests: the real `pyfaro` binary against real uv, Ruff, ty and
//! pytest in disposable directories. See `harness.rs` for the shared API and
//! CONTRIBUTING.md for the policy.
//!
//! The suite only runs with `PYFARO_E2E=1`; otherwise each test prints a skip
//! note and passes. Add one module per command group (`tests/e2e/<group>.rs`)
//! and one `mod` line below. Start every test with `e2e_guard!();`.

#[macro_use]
mod harness;

mod cli;
mod create_init;
mod environment;
mod project_env;
mod project_env;
mod quality;
