use std::fs;
use std::io::Write;
use std::process::{Command, Stdio};

use tempfile::tempdir;

#[test]
fn run_preserves_arguments_and_uses_the_project_environment() {
    let sandbox = tempdir().unwrap();
    let project = sandbox.path().join("project with spaces é");
    fs::create_dir(&project).unwrap();
    fs::write(
        project.join("pyproject.toml"),
        "[project]\nname = 'run-example'\nversion = '1.0.0'\nrequires-python = '>=3.10'\ndependencies = []\n",
    )
    .unwrap();
    let cache = sandbox.path().join("uv-cache");
    let script = "import os, sys; print('ARGS=' + '|'.join(sys.argv[1:])); print('PREFIX=' + sys.prefix); print('VENV=' + os.environ.get('VIRTUAL_ENV', ''))";
    let output = Command::new(env!("CARGO_BIN_EXE_pyfaro"))
        .args(["run", "--offline", "--project"])
        .arg(&project)
        .args(["--", "python", "-c", script, "a b", "-n", "café", "--"])
        .env("UV_CACHE_DIR", &cache)
        .env("UV_ISOLATED", "1")
        .env("PYTHONIOENCODING", "utf-8")
        .output()
        .unwrap();
    assert!(output.status.success(), "{output:?}");
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("ARGS=a b|-n|café|--"), "{stdout}");
    for label in ["PREFIX=", "VENV="] {
        let path = stdout
            .lines()
            .find_map(|line| line.strip_prefix(label))
            .unwrap_or_else(|| panic!("missing {label} in {stdout}"));
        assert_eq!(
            fs::canonicalize(path).unwrap(),
            fs::canonicalize(project.join(".venv")).unwrap()
        );
    }
    assert!(project.join("uv.lock").exists());

    let lock = fs::read(project.join("uv.lock")).unwrap();
    fs::write(
        project.join("pyproject.toml"),
        "[project]\nname = 'run-example'\nversion = '2.0.0'\nrequires-python = '>=3.10'\ndependencies = []\n",
    )
    .unwrap();
    let stale = Command::new(env!("CARGO_BIN_EXE_pyfaro"))
        .args(["run", "--locked", "--offline", "--project"])
        .arg(&project)
        .args(["--", "python", "-c", "print('should not run')"])
        .env("UV_CACHE_DIR", &cache)
        .output()
        .unwrap();
    assert!(!stale.status.success(), "{stale:?}");
    assert!(!stale.stdout.starts_with(b"should not run"));
    assert_eq!(fs::read(project.join("uv.lock")).unwrap(), lock);

    let failed = Command::new(env!("CARGO_BIN_EXE_pyfaro"))
        .args(["run", "--offline", "--project"])
        .arg(&project)
        .args([
            "--",
            "python",
            "-c",
            "import sys; print('failed'); sys.exit(23)",
        ])
        .env("UV_CACHE_DIR", &cache)
        .output()
        .unwrap();
    assert_eq!(failed.status.code(), Some(23), "{failed:?}");
    assert_eq!(String::from_utf8_lossy(&failed.stdout).trim_end(), "failed");
}

#[test]
fn run_uses_the_project_environment_for_inline_metadata_scripts() {
    let project = tempdir().unwrap();
    fs::write(
        project.path().join("pyproject.toml"),
        "[project]\nname = 'script-example'\nversion = '1.0.0'\nrequires-python = '>=3.10'\ndependencies = []\n",
    )
    .unwrap();
    fs::write(
        project.path().join("-task.py"),
        "# /// script\n# dependencies = []\n# ///\nimport sys\nprint(sys.prefix)\n",
    )
    .unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_pyfaro"))
        .args(["run", "--offline", "--project"])
        .arg(project.path())
        .args(["--", "./-task.py"])
        .current_dir(project.path())
        .env("UV_CACHE_DIR", project.path().join("uv-cache"))
        .output()
        .unwrap();
    assert!(output.status.success(), "{output:?}");
    let prefix = String::from_utf8(output.stdout).unwrap();
    assert_eq!(
        fs::canonicalize(prefix.trim()).unwrap(),
        fs::canonicalize(project.path().join(".venv")).unwrap()
    );
    let external = tempdir().unwrap();
    let external_script = external.path().join("outside.py");
    fs::write(
        &external_script,
        "# /// script\n# dependencies = []\n# ///\nimport sys\nprint(sys.prefix)\n",
    )
    .unwrap();
    let outside = Command::new(env!("CARGO_BIN_EXE_pyfaro"))
        .args(["run", "--locked", "--offline", "--project"])
        .arg(project.path())
        .arg("--")
        .arg(&external_script)
        .env("UV_CACHE_DIR", project.path().join("uv-cache"))
        .output()
        .unwrap();
    assert!(outside.status.success(), "{outside:?}");
    let outside_prefix = String::from_utf8(outside.stdout).unwrap();
    assert_eq!(
        fs::canonicalize(outside_prefix.trim()).unwrap(),
        fs::canonicalize(project.path().join(".venv")).unwrap()
    );
}

#[cfg(any(target_os = "linux", target_os = "macos"))]
#[test]
fn run_preserves_path_lookup_for_python_named_executables() {
    use std::os::unix::fs::PermissionsExt;

    let project = tempdir().unwrap();
    fs::write(
        project.path().join("pyproject.toml"),
        "[project]\nname = 'path-example'\nversion = '1.0.0'\nrequires-python = '>=3.10'\ndependencies = []\n",
    )
    .unwrap();
    let cache = project.path().join("uv-cache");
    let setup = Command::new(env!("CARGO_BIN_EXE_pyfaro"))
        .args(["run", "--offline", "--project"])
        .arg(project.path())
        .args(["--", "python", "-c", "pass"])
        .env("UV_CACHE_DIR", &cache)
        .output()
        .unwrap();
    assert!(setup.status.success(), "{setup:?}");
    let executable = project.path().join(".venv/bin/tool.py");
    fs::write(&executable, "#!/bin/sh\nprintf 'PATH_OK\\n'\n").unwrap();
    fs::set_permissions(&executable, fs::Permissions::from_mode(0o755))
        .unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_pyfaro"))
        .args(["run", "--locked", "--offline", "--project"])
        .arg(project.path())
        .args(["--", "tool.py"])
        .current_dir(project.path())
        .env("UV_CACHE_DIR", &cache)
        .output()
        .unwrap();
    assert!(output.status.success(), "{output:?}");
    assert_eq!(String::from_utf8_lossy(&output.stdout).trim_end(), "PATH_OK");
    fs::write(project.path().join("tool.py"), "print('PROJECT_FILE')\n")
        .unwrap();
    let local = Command::new(env!("CARGO_BIN_EXE_pyfaro"))
        .args(["run", "--locked", "--offline", "--project"])
        .arg(project.path())
        .args(["--", "tool.py"])
        .current_dir(project.path())
        .env("UV_CACHE_DIR", &cache)
        .output()
        .unwrap();
    assert!(local.status.success(), "{local:?}");
    assert_eq!(
        String::from_utf8_lossy(&local.stdout).trim_end(),
        "PROJECT_FILE"
    );
}

#[test]
fn run_rejects_a_directory_without_project_metadata() {
    let parent = tempdir().unwrap();
    fs::write(
        parent.path().join("pyproject.toml"),
        "[project]\nname = 'parent-example'\nversion = '1.0.0'\nrequires-python = '>=3.10'\n",
    )
    .unwrap();
    let child = parent.path().join("child");
    fs::create_dir(&child).unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_pyfaro"))
        .args(["run", "--locked", "--offline", "--project"])
        .arg(&child)
        .args(["--", "python", "-c", "print('should not run')"])
        .env("UV_CACHE_DIR", parent.path().join("uv-cache"))
        .output()
        .unwrap();
    assert!(!output.status.success(), "{output:?}");
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("pyproject.toml"),
        "{output:?}"
    );
    assert!(output.stdout.is_empty());
    assert!(!parent.path().join("uv.lock").exists());
    assert!(!parent.path().join(".venv").exists());
    fs::create_dir(child.join("pyproject.toml")).unwrap();
    let invalid = Command::new(env!("CARGO_BIN_EXE_pyfaro"))
        .args(["run", "--offline", "--project"])
        .arg(&child)
        .args(["--", "python", "-c", "pass"])
        .output()
        .unwrap();
    assert!(!invalid.status.success(), "{invalid:?}");
    assert!(
        String::from_utf8_lossy(&invalid.stderr)
            .contains("project metadata is not a file"),
        "{invalid:?}"
    );
}

#[test]
fn run_requires_the_argument_separator() {
    let output = Command::new(env!("CARGO_BIN_EXE_pyfaro"))
        .args(["run", "python"])
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(2));
}

#[test]
fn run_forwards_redirected_input() {
    let project = tempdir().unwrap();
    fs::write(
        project.path().join("pyproject.toml"),
        "[project]\nname = 'input-example'\nversion = '1.0.0'\nrequires-python = '>=3.10'\ndependencies = []\n",
    )
    .unwrap();
    let mut child = Command::new(env!("CARGO_BIN_EXE_pyfaro"))
        .args(["run", "--offline", "--project"])
        .arg(project.path())
        .args(["--", "python", "-c", "print(input().upper())"])
        .env("UV_CACHE_DIR", project.path().join("uv-cache"))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    child.stdin.take().unwrap().write_all(b"hello\n").unwrap();
    let output = child.wait_with_output().unwrap();
    assert!(output.status.success(), "{output:?}");
    assert_eq!(String::from_utf8_lossy(&output.stdout).trim_end(), "HELLO");
}

#[cfg(any(target_os = "linux", target_os = "macos"))]
#[test]
fn run_accepts_input_from_a_foreground_terminal() {
    let (success, output) =
        run_in_terminal("print(input().upper())", Some(b"hello\r"));
    assert!(success, "{output}");
    assert!(output.contains("HELLO"), "{output}");
}

#[cfg(any(target_os = "linux", target_os = "macos"))]
#[test]
fn run_reports_terminal_device_access_instead_of_hanging() {
    let (success, output) = run_in_terminal(
        "print('ready', flush=True); open('/dev/tty').readline()",
        None,
    );
    assert!(!success, "{output}");
    assert!(output.contains("delegated command stopped"), "{output}");
}

#[cfg(any(target_os = "linux", target_os = "macos"))]
fn run_in_terminal(script: &str, input: Option<&[u8]>) -> (bool, String) {
    use std::io::Read;
    use std::sync::mpsc;
    use std::thread;
    use std::time::Duration;

    use portable_pty::{CommandBuilder, PtySize, native_pty_system};

    let project = tempdir().unwrap();
    fs::write(
        project.path().join("pyproject.toml"),
        "[project]\nname = 'terminal-example'\nversion = '1.0.0'\nrequires-python = '>=3.10'\ndependencies = []\n",
    )
    .unwrap();
    let pair = native_pty_system()
        .openpty(PtySize {
            rows: 24,
            cols: 80,
            pixel_width: 0,
            pixel_height: 0,
        })
        .unwrap();
    let mut command = CommandBuilder::new(env!("CARGO_BIN_EXE_pyfaro"));
    for argument in ["run", "--offline", "--project"] {
        command.arg(argument);
    }
    command.arg(project.path());
    for argument in ["--", "python", "-c", script] {
        command.arg(argument);
    }
    command.env("UV_CACHE_DIR", project.path().join("uv-cache"));
    let mut child = pair.slave.spawn_command(command).unwrap();
    let mut killer = child.clone_killer();
    let mut reader = pair.master.try_clone_reader().unwrap();
    let mut writer = pair.master.take_writer().unwrap();
    let (status_sender, status_receiver) = mpsc::channel();
    thread::spawn(move || {
        let _ = status_sender.send(child.wait());
    });
    let (output_sender, output_receiver) = mpsc::channel();
    thread::spawn(move || {
        let mut output = Vec::new();
        let _ = reader.read_to_end(&mut output);
        let _ = output_sender.send(output);
    });
    if let Some(input) = input {
        writer.write_all(input).unwrap();
    }
    let status = match status_receiver.recv_timeout(Duration::from_secs(10)) {
        Ok(status) => status.unwrap(),
        Err(error) => {
            let _ = killer.kill();
            panic!("terminal command did not exit: {error}");
        }
    };
    drop(writer);
    drop(pair.slave);
    let output = output_receiver.recv_timeout(Duration::from_secs(3)).unwrap();
    (status.success(), String::from_utf8_lossy(&output).into_owned())
}
