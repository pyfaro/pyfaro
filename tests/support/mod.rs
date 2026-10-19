use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::OnceLock;

struct Fixture {
    _directory: tempfile::TempDir,
    executable: PathBuf,
}

/// Compile a portable external uv stand-in for failure/cancellation scenarios.
/// Successful workflow coverage uses the real, pinned uv executable instead.
pub fn fixture_uv() -> &'static Path {
    static FIXTURE: OnceLock<Fixture> = OnceLock::new();
    &FIXTURE
        .get_or_init(|| {
            let directory = tempfile::tempdir().unwrap();
            let executable = directory
                .path()
                .join(format!("fixture-uv{}", std::env::consts::EXE_SUFFIX));
            let source = Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("tests/support/uv.rs");
            let output = Command::new(
                std::env::var_os("RUSTC").unwrap_or_else(|| "rustc".into()),
            )
            .arg(source)
            .args(["--edition", "2024", "-o"])
            .arg(&executable)
            .output()
            .expect("compile the external uv fixture");
            assert!(output.status.success(), "{output:?}");
            Fixture { _directory: directory, executable }
        })
        .executable
}
