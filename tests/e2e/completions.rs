//! `pyfaro completions <shell>`: real scripts checked with the real shells.

use std::process::Command;

use crate::harness::Sandbox;

const SHELLS: [&str; 4] = ["bash", "zsh", "fish", "powershell"];

/// Subcommands that every script must offer.
const SOME_COMMANDS: [&str; 6] =
    ["create", "sync", "run", "check", "test", "completions"];

/// Print the script for `shell` into the sandbox and return its path.
fn script(sandbox: &Sandbox, shell: &str) -> std::path::PathBuf {
    let out = sandbox.pyfaro(["completions", shell]).run();
    out.success().stderr_empty();
    sandbox.write(format!("pyfaro.{shell}"), &out.stdout)
}

/// Run a shell and fail with its output when it does not exit 0.
fn shell_succeeds(mut command: Command, what: &str) -> String {
    let output = command
        .output()
        .unwrap_or_else(|error| panic!("{what}: cannot start: {error}"));
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    assert!(
        output.status.success(),
        "{what} failed ({}):\n{stdout}\n{}",
        output.status,
        String::from_utf8_lossy(&output.stderr)
    );
    stdout
}

#[test]
fn every_shell_prints_a_script_with_the_subcommands() {
    e2e_guard!();
    let sandbox = Sandbox::new();
    for shell in SHELLS {
        let out = sandbox.pyfaro(["completions", shell]).run();
        out.success().stderr_empty();
        assert!(!out.stdout.is_empty(), "{shell}: empty script");
        for command in SOME_COMMANDS {
            out.stdout_contains(command);
        }
    }
}

#[test]
fn completions_need_no_project_and_no_uv() {
    e2e_guard!();
    let sandbox = Sandbox::new();
    let empty_path = sandbox.join("empty-path");
    std::fs::create_dir_all(&empty_path).unwrap();
    for shell in SHELLS {
        sandbox
            .pyfaro(["completions", shell])
            .env("PATH", &empty_path)
            .run()
            .success()
            .stderr_empty();
    }
}

#[test]
fn an_unknown_shell_is_a_usage_error() {
    e2e_guard!();
    let sandbox = Sandbox::new();
    sandbox
        .pyfaro(["completions", "tcsh"])
        .run()
        .code(2)
        .stderr_contains("invalid value 'tcsh'")
        .stderr_contains("[possible values: bash, zsh, fish, powershell]")
        .stdout_empty();
}

#[test]
fn a_missing_shell_is_a_usage_error() {
    e2e_guard!();
    let sandbox = Sandbox::new();
    sandbox
        .pyfaro(["completions"])
        .run()
        .code(2)
        .stderr_contains("<SHELL>")
        .stdout_empty();
}

#[test]
fn completions_documents_its_examples() {
    e2e_guard!();
    let sandbox = Sandbox::new();
    sandbox
        .pyfaro(["completions", "--help"])
        .run()
        .success()
        .stdout_contains("Examples:\n  pyfaro completions bash")
        .stdout_contains("https://pyfaro.dev/commands/completions/");
}

#[cfg(unix)]
#[test]
fn the_bash_script_parses() {
    e2e_guard!();
    let sandbox = Sandbox::new();
    let script = script(&sandbox, "bash");
    shell_succeeds(
        {
            let mut command = Command::new("bash");
            command.arg("-n").arg(&script);
            command
        },
        "bash -n",
    );
}

#[cfg(unix)]
#[test]
fn the_zsh_script_parses() {
    e2e_guard!();
    let sandbox = Sandbox::new();
    let script = script(&sandbox, "zsh");
    shell_succeeds(
        {
            let mut command = Command::new("zsh");
            command.arg("-n").arg(&script);
            command
        },
        "zsh -n (install zsh to run this test)",
    );
}

#[cfg(unix)]
#[test]
fn the_fish_script_parses() {
    e2e_guard!();
    let sandbox = Sandbox::new();
    let script = script(&sandbox, "fish");
    shell_succeeds(
        {
            let mut command = Command::new("fish");
            command.arg("--no-execute").arg(&script);
            command
        },
        "fish --no-execute (install fish to run this test)",
    );
}

#[cfg(windows)]
#[test]
fn the_powershell_script_parses() {
    e2e_guard!();
    let sandbox = Sandbox::new();
    let script = script(&sandbox, "powershell");
    let parse = "$tokens = $null; $errors = $null; \
        $text = Get-Content -Raw -Encoding UTF8 -LiteralPath $env:PYFARO_SCRIPT; \
        [System.Management.Automation.Language.Parser]::ParseInput(\
        $text, [ref]$tokens, [ref]$errors) | Out-Null; \
        if ($errors.Count -gt 0) { $errors | ForEach-Object { $_.Message }; exit 1 }";
    shell_succeeds(
        {
            let mut command = Command::new("powershell.exe");
            command
                .args(["-NoProfile", "-NonInteractive", "-Command", parse])
                .env("PYFARO_SCRIPT", &script);
            command
        },
        "PowerShell parser",
    );
}

/// Complete `words` (the last one is being typed) with the generated bash
/// function, driven through the variables bash itself sets.
#[cfg(unix)]
fn bash_complete(script: &std::path::Path, words: &[&str]) -> Vec<String> {
    const DRIVER: &str = r#"
        source "$1"
        cword=$2
        shift 2
        COMP_WORDS=("$@")
        COMP_CWORD=$cword
        COMP_LINE="$*"
        COMP_POINT=${#COMP_LINE}
        _pyfaro "${COMP_WORDS[0]}" "${COMP_WORDS[$cword]}" \
            "${COMP_WORDS[$((cword - 1))]}"
        printf '%s\n' "${COMPREPLY[@]}"
    "#;
    let mut command = Command::new("bash");
    command
        .args(["--noprofile", "--norc", "-c", DRIVER, "pyfaro-driver"])
        .arg(script)
        .arg((words.len() - 1).to_string())
        .args(words);
    shell_succeeds(command, "bash completion")
        .lines()
        .map(str::to_owned)
        .collect()
}

#[cfg(unix)]
#[test]
fn bash_completes_subcommands_flags_and_values() {
    e2e_guard!();
    let sandbox = Sandbox::new();
    let script = script(&sandbox, "bash");

    assert_eq!(bash_complete(&script, &["pyfaro", "sy"]), ["sync"]);
    assert_eq!(bash_complete(&script, &["pyfaro", "comp"]), ["completions"]);
    let all = bash_complete(&script, &["pyfaro", ""]);
    for command in ["create", "sync", "run", "dev", "check", "completions"] {
        assert!(all.iter().any(|word| word == command), "{command}: {all:?}");
    }
    assert_eq!(
        bash_complete(&script, &["pyfaro", "sync", "--of"]),
        ["--offline"]
    );
    let shells = bash_complete(&script, &["pyfaro", "completions", ""]);
    for shell in SHELLS {
        assert!(shells.iter().any(|word| word == shell), "{shell}: {shells:?}");
    }
    assert_eq!(bash_complete(&script, &["pyfaro", "create", "l"]), ["lib"]);
}
