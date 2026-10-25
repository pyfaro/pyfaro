//! Global CLI behavior: help, version and usage errors.

use crate::harness::Sandbox;

/// Every implemented subcommand, as listed by `pyfaro --help`.
const COMMANDS: [&str; 16] = [
    "create",
    "init",
    "sync",
    "add",
    "remove",
    "run",
    "dev",
    "lint",
    "fmt",
    "type",
    "test",
    "check",
    "python",
    "tools",
    "doctor",
    "completions",
];

#[test]
fn help_lists_every_implemented_command() {
    e2e_guard!();
    let sandbox = Sandbox::new();
    let out = sandbox.pyfaro(["--help"]).run();
    out.success().stdout_contains("Usage: pyfaro").stderr_empty();
    for command in COMMANDS {
        out.stdout_contains(&format!("\n  {command} "));
    }
}

#[test]
fn version_prints_the_package_version() {
    e2e_guard!();
    let sandbox = Sandbox::new();
    sandbox
        .pyfaro(["--version"])
        .run()
        .success()
        .stdout_eq(&format!("pyfaro {}\n", env!("CARGO_PKG_VERSION")))
        .stderr_empty();
}

#[test]
fn no_arguments_prints_help_and_exits_2() {
    e2e_guard!();
    let sandbox = Sandbox::new();
    sandbox
        .pyfaro(Vec::<&str>::new())
        .run()
        .code(2)
        .stderr_contains("Usage: pyfaro")
        .stdout_empty();
}

#[test]
fn an_unknown_subcommand_is_a_usage_error() {
    e2e_guard!();
    let sandbox = Sandbox::new();
    sandbox
        .pyfaro(["frobnicate"])
        .run()
        .code(2)
        .stderr_contains("unrecognized subcommand 'frobnicate'")
        .stdout_empty();
}

#[test]
fn an_unknown_flag_is_a_usage_error() {
    e2e_guard!();
    let sandbox = Sandbox::new();
    sandbox
        .pyfaro(["--frobnicate"])
        .run()
        .code(2)
        .stderr_contains("unexpected argument '--frobnicate'")
        .stdout_empty();
}

#[test]
fn help_subcommand_and_help_flag_work_for_every_command() {
    e2e_guard!();
    let sandbox = Sandbox::new();
    for command in COMMANDS {
        let via_help = sandbox.pyfaro(["help", command]).run();
        via_help.success().stdout_contains(&format!(
            "Usage: pyfaro{} {command}",
            std::env::consts::EXE_SUFFIX
        ));
        let via_flag = sandbox.pyfaro([command, "--help"]).run();
        via_flag.success().stdout_eq(&via_help.stdout);
    }
}

#[test]
fn top_level_help_shows_the_workflow_and_the_website() {
    e2e_guard!();
    let sandbox = Sandbox::new();
    let out = sandbox.pyfaro(["--help"]).run();
    out.success()
        .stdout_contains("\nWorkflow:\n")
        .stdout_contains("  pyfaro create cli acme-tools")
        .stdout_contains("  pyfaro sync ")
        .stdout_contains("  pyfaro run -- python --version")
        .stdout_contains("  pyfaro check --fix")
        .stdout_contains("  pyfaro test ")
        .stdout_contains("  pyfaro dev ")
        .stdout_contains("https://pyfaro.dev/")
        .stderr_empty();
    assert!(!out.stdout.contains("not implemented yet"));
}

#[test]
fn subcommand_help_shows_examples() {
    e2e_guard!();
    let sandbox = Sandbox::new();
    for (args, example) in [
        (["create", "--help"], "pyfaro create lib acme-widget"),
        (["sync", "--help"], "pyfaro sync --locked"),
        (["add", "--help"], "pyfaro add --dev pytest-cov"),
        (["run", "--help"], "pyfaro run -- python -m pytest"),
        (["dev", "--help"], "pyfaro dev -- --port 9000"),
        (["test", "--help"], "pyfaro test -k parser -x"),
        (["check", "--help"], "pyfaro check --fix"),
        (["python", "--help"], "pyfaro python install 3.12"),
        (["completions", "--help"], "pyfaro completions zsh"),
    ] {
        sandbox
            .pyfaro(args)
            .run()
            .success()
            .stdout_contains("\nExamples:\n")
            .stdout_contains(&format!("  {example}"));
    }
}
