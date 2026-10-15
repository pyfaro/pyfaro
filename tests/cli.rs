use std::process::Command;

#[test]
fn help_describes_the_current_development_status() {
    let output = Command::new(env!("CARGO_BIN_EXE_pyfaro"))
        .arg("--help")
        .output()
        .expect("CLI should start");

    assert!(output.status.success());
    let stdout =
        String::from_utf8(output.stdout).expect("help should be UTF-8");
    assert!(stdout.contains("Usage: pyfaro"));
    assert!(stdout.contains("not implemented yet"));
    assert!(output.stderr.is_empty());
}

#[test]
fn version_matches_the_cargo_package() {
    let output = Command::new(env!("CARGO_BIN_EXE_pyfaro"))
        .arg("--version")
        .output()
        .expect("CLI should start");

    assert!(output.status.success());
    assert_eq!(
        String::from_utf8(output.stdout).expect("version should be UTF-8"),
        format!("pyfaro {}\n", env!("CARGO_PKG_VERSION"))
    );
}

#[test]
fn unimplemented_commands_fail_with_usage_guidance() {
    let output = Command::new(env!("CARGO_BIN_EXE_pyfaro"))
        .arg("create")
        .output()
        .expect("CLI should start");

    assert_eq!(output.status.code(), Some(2));
    let stderr =
        String::from_utf8(output.stderr).expect("error should be UTF-8");
    assert!(stderr.contains("error:"));
    assert!(stderr.contains("--help"));
    assert!(output.stdout.is_empty());
}
