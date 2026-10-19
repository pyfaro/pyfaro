use std::fs;
use std::process::Command;

use tempfile::tempdir;

#[test]
fn invalid_configuration_fails_before_uv_for_run_and_sync() {
    let directory = tempdir().unwrap();
    let metadata = directory.path().join("pyproject.toml");
    for (setting, expected) in [
        ("schema-version = 2", "schema-version"),
        ("schema-version = '1'", "schema-version"),
        ("surprise = true", "unknown [tool.pyfaro] setting"),
    ] {
        fs::write(
            &metadata,
            format!(
                "[project]\nname = 'example'\nversion = '1.0.0'\n\n[tool.pyfaro]\n{setting}\n"
            ),
        )
        .unwrap();
        for subcommand in ["run", "sync"] {
            let mut command = Command::new(env!("CARGO_BIN_EXE_pyfaro"));
            command.arg(subcommand).arg("--uv").arg("missing-uv-binary");
            if subcommand == "run" {
                command.args(["--", "python", "--version"]);
            }
            let output =
                command.current_dir(directory.path()).output().unwrap();
            assert!(!output.status.success(), "{output:?}");
            let diagnostic = String::from_utf8_lossy(&output.stderr);
            assert!(diagnostic.contains(expected), "{diagnostic}");
            assert!(diagnostic.contains("pyproject.toml"), "{diagnostic}");
            assert!(!diagnostic.contains("missing-uv-binary"), "{diagnostic}");
        }
    }
}
