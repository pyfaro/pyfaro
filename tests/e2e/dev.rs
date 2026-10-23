//! `pyfaro dev`: `[tool.pyfaro.dev]` validation, module mode, the FastAPI
//! preset and supervision.
//!
//! Module-mode projects are virtual (no dependencies, no build system), so
//! those tests need no network. The FastAPI tests sync real FastAPI and
//! Uvicorn and serve real HTTP on a free port.

use std::net::TcpListener;
use std::path::PathBuf;
#[cfg(windows)]
use std::thread;
use std::time::{Duration, Instant};

use crate::harness::{
    Cmd, SHORT, STARTUP, Sandbox, free_port, process_exists,
    wait_until_port_is_free, wait_until_process_is_gone,
};

const GUIDE: &str = "https://pyfaro.dev/commands/dev/";

/// A virtual project with `demo/worker.py` and `demo/serve.py` at its root
/// and the given extra `pyproject.toml` text.
fn module_project(sandbox: &Sandbox, extra: &str) -> PathBuf {
    sandbox.write(
        "project/pyproject.toml",
        &format!(
            "[project]\nname = \"demo\"\nversion = \"0.1.0\"\n\
             requires-python = \">=3.10\"\ndependencies = []\n\n\
             [tool.pyfaro]\nschema-version = 1\n\n{extra}\n"
        ),
    );
    sandbox.write("project/demo/__init__.py", "");
    sandbox.write(
        "project/demo/worker.py",
        "import os\nimport sys\n\n\
         print('args=' + repr(sys.argv[1:]))\n\
         print('cwd=' + os.path.basename(os.getcwd()))\n\
         print('hello', flush=True)\n\
         print('child-err', file=sys.stderr, flush=True)\n\
         raise SystemExit(3 if '--fail' in sys.argv else 0)\n",
    );
    sandbox.write(
        "project/demo/serve.py",
        "import os\nimport signal\nimport sys\nimport time\n\n\
         stopping = []\n\n\
         print(f'pid={os.getpid()}', flush=True)\n\
         mode = sys.argv[1] if len(sys.argv) > 1 else ''\n\
         # Repeated interrupts (the process group's and uv's forwarded one)\n\
         # only record the request; the cleanup below must not be cut short.\n\
         signal.signal(\n\
         \x20   signal.SIGINT,\n\
         \x20   signal.SIG_IGN if mode == 'stubborn' else lambda *_: stopping.append(1),\n\
         )\n\
         print('ready', flush=True)\n\
         while not stopping:\n    time.sleep(0.1)\n\
         time.sleep(1.5)\n\
         open('cleanup.txt', 'w').write('done')\n",
    );
    sandbox.join("project")
}

const WORKER: &str =
    "[tool.pyfaro.dev]\nentry-point = \"demo.worker\"\nargs = [\"--first\"]\n";
const SERVE: &str = "[tool.pyfaro.dev]\nentry-point = \"demo.serve\"\n";

fn pid_line(stdout: &str) -> u32 {
    stdout
        .lines()
        .find_map(|line| line.trim().strip_prefix("pid="))
        .expect("pid line")
        .parse()
        .expect("pid number")
}

/// A FastAPI project: `demo.main:app` records every process that imports it
/// in `pids.txt` and answers `GET /`.
fn fastapi_project(sandbox: &Sandbox, dev: &str) -> PathBuf {
    sandbox.write(
        "project/pyproject.toml",
        &format!(
            "[project]\nname = \"demo\"\nversion = \"0.1.0\"\n\
             requires-python = \">=3.10\"\n\
             dependencies = [\"fastapi>=0.110\", \"uvicorn[standard]>=0.30\"]\n\n\
             [build-system]\nrequires = [\"uv_build>=0.9.26,<0.10.0\"]\n\
             build-backend = \"uv_build\"\n\n\
             [tool.pyfaro]\nschema-version = 1\n\n{dev}\n"
        ),
    );
    sandbox.write("project/src/demo/__init__.py", "");
    sandbox.write(
        "project/src/demo/main.py",
        "import os\nfrom pathlib import Path\n\nfrom fastapi import FastAPI\n\n\
         with open('pids.txt', 'a') as pids:\n    pids.write(f'{os.getpid()}\\n')\n\n\
         app = FastAPI()\n\n\n\
         @app.get('/')\ndef root() -> dict[str, str]:\n\
         \x20   return {'message': 'hello', 'cwd': Path.cwd().name}\n",
    );
    sandbox.join("project")
}

fn recorded_pids(root: &std::path::Path) -> Vec<u32> {
    std::fs::read_to_string(root.join("pids.txt"))
        .unwrap_or_default()
        .lines()
        .filter_map(|line| line.trim().parse().ok())
        .collect()
}

/// Every process the app recorded is gone.
fn assert_no_survivors(root: &std::path::Path) {
    let pids = recorded_pids(root);
    assert!(!pids.is_empty(), "the app never started");
    for pid in pids {
        wait_until_process_is_gone(pid, SHORT);
    }
}

/// Stop a live server like a user would and check that the port is released.
fn stop_and_assert_clean(
    mut live: crate::harness::Live,
    port: u16,
    root: &std::path::Path,
) {
    live.interrupt();
    let out = live.finish(SHORT);
    out.code(130).stderr_contains("pyfaro dev: stopped");
    assert_no_survivors(root);
    assert!(
        wait_until_port_is_free(port, SHORT),
        "port {port} is still bound after stopping"
    );
}

// ---------------------------------------------------------- configuration

#[test]
fn a_missing_dev_table_prints_a_hint_and_exits_1() {
    e2e_guard!();
    let sandbox = Sandbox::new();
    let root = module_project(&sandbox, "");
    let out = Cmd::pyfaro(&root, ["dev"]).run();
    out.code(1)
        .stdout_empty()
        .stderr_contains("pyfaro dev: no [tool.pyfaro.dev] table in ")
        .stderr_contains("pyproject.toml")
        .stderr_contains("[tool.pyfaro.dev]\n")
        .stderr_contains("preset = \"fastapi\"")
        .stderr_contains("entry-point = ")
        .stderr_contains(GUIDE);
    assert!(!root.join(".venv").exists(), "nothing may be launched");
}

fn assert_rejected(table: &str, message: &str) {
    let sandbox = Sandbox::new();
    let root = module_project(&sandbox, table);
    let out = Cmd::pyfaro(&root, ["dev"]).run();
    out.code(1)
        .stdout_empty()
        .stderr_contains("error: invalid ")
        .stderr_contains("pyproject.toml")
        .stderr_contains(message);
    assert!(!root.join(".venv").exists(), "nothing may be launched: {table}");
}

#[test]
fn an_unknown_dev_key_is_rejected() {
    e2e_guard!();
    assert_rejected(
        "[tool.pyfaro.dev]\nentry-point = \"demo.worker\"\nwatch = true\n",
        "unknown [tool.pyfaro.dev] setting: watch",
    );
}

#[test]
fn a_missing_entry_point_is_rejected() {
    e2e_guard!();
    assert_rejected(
        "[tool.pyfaro.dev]\nargs = []\n",
        "tool.pyfaro.dev.entry-point is required",
    );
    assert_rejected(
        "[tool.pyfaro.dev]\nentry-point = 5\n",
        "tool.pyfaro.dev.entry-point must be a string",
    );
}

#[test]
fn a_bad_preset_is_rejected() {
    e2e_guard!();
    assert_rejected(
        "[tool.pyfaro.dev]\npreset = \"flask\"\nentry-point = \"demo.main:app\"\n",
        "tool.pyfaro.dev.preset must be the string \"fastapi\"",
    );
    assert_rejected(
        "[tool.pyfaro.dev]\npreset = true\nentry-point = \"demo.main:app\"\n",
        "tool.pyfaro.dev.preset must be the string \"fastapi\"",
    );
}

#[test]
fn module_mode_entry_points_are_module_names() {
    e2e_guard!();
    for entry in ["demo.main:app", "demo..worker", "1demo", "", "demo worker"] {
        assert_rejected(
            &format!("[tool.pyfaro.dev]\nentry-point = {entry:?}\n"),
            "tool.pyfaro.dev.entry-point must be a module name",
        );
    }
}

#[test]
fn fastapi_entry_points_are_asgi_import_strings() {
    e2e_guard!();
    for entry in [
        "demo.main",
        "demo.main:",
        ":app",
        "demo:app:x",
        "demo:a.b",
        "a..b:app",
    ] {
        assert_rejected(
            &format!(
                "[tool.pyfaro.dev]\npreset = \"fastapi\"\nentry-point = {entry:?}\n"
            ),
            "tool.pyfaro.dev.entry-point must be an ASGI import string",
        );
    }
}

#[test]
fn args_must_be_an_array_of_strings() {
    e2e_guard!();
    for args in ["\"--once\"", "[\"--once\", 1]", "{ a = \"b\" }"] {
        assert_rejected(
            &format!(
                "[tool.pyfaro.dev]\nentry-point = \"demo.worker\"\nargs = {args}\n"
            ),
            "tool.pyfaro.dev.args must be an array of strings",
        );
    }
}

#[test]
fn reload_needs_a_preset_and_a_boolean() {
    e2e_guard!();
    assert_rejected(
        "[tool.pyfaro.dev]\nentry-point = \"demo.worker\"\nreload = true\n",
        "reload requires preset = \"fastapi\"; Pyfaro does not watch files itself",
    );
    assert_rejected(
        "[tool.pyfaro.dev]\npreset = \"fastapi\"\nentry-point = \"demo.main:app\"\nreload = \"yes\"\n",
        "tool.pyfaro.dev.reload must be a boolean",
    );
}

#[test]
fn an_invalid_dev_table_is_reported_by_every_command() {
    e2e_guard!();
    let sandbox = Sandbox::new();
    let root = module_project(
        &sandbox,
        "[tool.pyfaro.dev]\nentry-point = \"demo.worker\"\nwatch = true\n",
    );
    Cmd::pyfaro(&root, ["sync"])
        .run()
        .code(1)
        .stderr_contains("unknown [tool.pyfaro.dev] setting: watch");
    assert!(!root.join(".venv").exists());
}

#[test]
fn other_commands_work_without_a_dev_table() {
    e2e_guard!();
    let sandbox = Sandbox::new();
    let root = module_project(&sandbox, "");
    Cmd::pyfaro(&root, ["run", "--", "python", "-c", "print('ok')"])
        .run()
        .success()
        .stdout_text_eq("ok\n");
}

#[test]
fn help_describes_the_flags() {
    e2e_guard!();
    let sandbox = Sandbox::new();
    let out = sandbox.pyfaro(["dev", "--help"]).run();
    out.success().stderr_empty();
    for flag in [
        "--project",
        "--locked",
        "--frozen",
        "--uv",
        "--offline",
        "-v, --verbose",
    ] {
        out.stdout_contains(flag);
    }
    out.stdout_contains("[EXTRA]...");
}

#[test]
fn locked_and_frozen_conflict() {
    e2e_guard!();
    let sandbox = Sandbox::new();
    sandbox
        .pyfaro(["dev", "--locked", "--frozen"])
        .run()
        .code(2)
        .stderr_contains("cannot be used with");
}

// ------------------------------------------------------------ module mode

#[test]
fn module_mode_runs_python_dash_m_in_the_project_root() {
    e2e_guard!();
    let sandbox = Sandbox::new();
    let root = module_project(&sandbox, WORKER);
    // Started from a subdirectory: discovery finds the project, the process
    // runs in the project root.
    let nested = root.join("demo");
    let out = Cmd::pyfaro(&nested, ["dev", "--", "second", "a b", ""]).run();
    out.success()
        .stdout_text_eq("args=['--first', 'second', 'a b', '']\ncwd=project\nhello\n")
        .stderr_contains("child-err")
        .stderr_contains(
            "pyfaro dev: starting python -m demo.worker (press Ctrl-C to stop)\n",
        );
    assert!(!out.stderr.contains("exited with status"), "{}", out.stderr);
    assert!(!out.stderr.contains("running "), "{}", out.stderr);
}

#[test]
fn child_output_is_not_prefixed_or_altered() {
    e2e_guard!();
    let sandbox = Sandbox::new();
    let root = module_project(&sandbox, WORKER);
    let out = Cmd::pyfaro(&root, ["dev"]).run();
    out.success();
    // Every Pyfaro line starts with the prefix; the child's never do.
    let own: Vec<&str> =
        out.stderr.lines().filter(|l| l.starts_with("pyfaro dev: ")).collect();
    assert_eq!(own.len(), 1, "{}", out.stderr);
    assert!(
        out.stderr.lines().any(|line| line == "child-err"),
        "{}",
        out.stderr
    );
    assert!(out.stdout.lines().all(|line| !line.starts_with("pyfaro")));
}

#[test]
fn a_failing_module_forwards_its_exit_status() {
    e2e_guard!();
    let sandbox = Sandbox::new();
    let root = module_project(&sandbox, WORKER);
    let out = Cmd::pyfaro(&root, ["dev", "--", "--fail"]).run();
    out.code(3)
        .stdout_contains("args=['--first', '--fail']")
        .stderr_contains("child-err")
        .stderr_contains("pyfaro dev: python exited with status 3\n");
    assert!(!out.stderr.contains("error:"), "{}", out.stderr);
}

#[test]
fn project_selects_the_project_from_anywhere() {
    e2e_guard!();
    let sandbox = Sandbox::new();
    module_project(&sandbox, WORKER);
    sandbox
        .pyfaro(["dev", "--project", "project"])
        .run()
        .success()
        .stdout_contains("cwd=project");
    let elsewhere = sandbox.join("elsewhere");
    std::fs::create_dir(&elsewhere).unwrap();
    Cmd::pyfaro(&elsewhere, ["dev", "--project", "../project"])
        .run()
        .success()
        .stdout_contains("cwd=project");
    // Without --project nothing is found from outside the project.
    Cmd::pyfaro(&elsewhere, ["dev"])
        .run()
        .code(1)
        .stderr_contains("no pyproject.toml found");
}

#[test]
fn verbose_prints_the_full_uv_command_line() {
    e2e_guard!();
    let sandbox = Sandbox::new();
    let root = module_project(&sandbox, WORKER);
    let out = Cmd::pyfaro(&root, ["dev", "--verbose", "--", "a b"]).run();
    out.success();
    let line = out
        .stderr
        .lines()
        .find(|line| line.starts_with("pyfaro dev: running "))
        .unwrap_or_else(|| panic!("no verbose line:\n{}", out.stderr));
    assert!(line.contains("uv"), "{line}");
    assert!(line.contains("--no-progress run --project "), "{line}");
    assert!(
        line.ends_with(" -- python -m demo.worker --first \"a b\""),
        "{line}"
    );
    // The verbose line follows the startup line.
    let starting = out.stderr.find("pyfaro dev: starting").unwrap();
    assert!(starting < out.stderr.find("pyfaro dev: running").unwrap());

    // Short flag and the lock options reach uv.
    Cmd::pyfaro(&root, ["sync"]).run().success();
    for lock in ["--locked", "--frozen"] {
        let out = Cmd::pyfaro(&root, ["dev", "-v", lock]).run();
        out.success()
            .stderr_contains(&format!(" {lock} -- python -m demo.worker"));
    }
}

#[test]
fn offline_reaches_uv() {
    e2e_guard!();
    let sandbox = Sandbox::new();
    let root = module_project(&sandbox, WORKER);
    Cmd::pyfaro(&root, ["dev", "--offline", "-v"])
        .run()
        .success()
        .stderr_contains("--no-progress --offline run --project ");
}

#[test]
fn ctrl_c_stops_a_module_and_leaves_no_process() {
    e2e_guard!();
    let sandbox = Sandbox::new();
    let root = module_project(&sandbox, SERVE);
    let mut live = Cmd::pyfaro(&root, ["dev"]).spawn();
    live.wait_for_stdout("ready", STARTUP);
    let pid = pid_line(&live.stdout());
    assert!(process_exists(pid));
    live.interrupt();
    let out = live.finish(SHORT);
    out.code(130).stderr_contains("pyfaro dev: stopped\n");
    assert!(!out.stderr.contains("error:"), "{}", out.stderr);
    wait_until_process_is_gone(pid, SHORT);
}

#[test]
fn a_cooperative_process_gets_time_to_shut_down() {
    e2e_guard!();
    let sandbox = Sandbox::new();
    let root = module_project(&sandbox, SERVE);
    let mut live = Cmd::pyfaro(&root, ["dev"]).spawn();
    live.wait_for_stdout("ready", STARTUP);
    live.interrupt();
    // The process needs 1.5 s after SIGINT, longer than the default 250 ms.
    live.finish(SHORT).code(130).stderr_contains("pyfaro dev: stopped\n");
    assert_eq!(
        std::fs::read_to_string(root.join("cleanup.txt")).as_deref().ok(),
        Some("done"),
        "the process was killed before it finished shutting down"
    );
}

#[cfg(unix)]
#[test]
fn a_process_that_ignores_ctrl_c_is_killed_after_the_grace_period() {
    e2e_guard!();
    let sandbox = Sandbox::new();
    let root = module_project(&sandbox, SERVE);
    let mut live = Cmd::pyfaro(&root, ["dev", "--", "stubborn"]).spawn();
    live.wait_for_stdout("ready", STARTUP);
    let pid = pid_line(&live.stdout());
    let interrupted = Instant::now();
    live.interrupt();
    let out = live.finish(SHORT);
    let waited = interrupted.elapsed();
    out.code(130).stderr_contains("pyfaro dev: stopped\n");
    assert!(
        waited >= Duration::from_secs(4) && waited < Duration::from_secs(20),
        "stopped after {waited:?}, expected about the 5 s grace period"
    );
    wait_until_process_is_gone(pid, SHORT);
}

/// The console delivers Ctrl-C to every attached process, so a process that
/// survives it was not asked to stop: Pyfaro keeps supervising it.
#[cfg(windows)]
#[test]
fn a_ctrl_c_the_process_survives_is_not_a_stop_request() {
    e2e_guard!();
    let sandbox = Sandbox::new();
    let root = module_project(&sandbox, SERVE);
    let mut live = Cmd::pyfaro(&root, ["dev", "--", "stubborn"]).spawn();
    live.wait_for_stdout("ready", STARTUP);
    let pid = pid_line(&live.stdout());
    live.interrupt();
    // Past the 5 s grace period.
    thread::sleep(Duration::from_secs(8));
    assert!(live.is_running(), "pyfaro dev stopped: {}", live.stderr());
    assert!(process_exists(pid));
    assert!(!live.stderr().contains("stopped"), "{}", live.stderr());
    // The window closed, so this is a first Ctrl-C again, then a second.
    live.interrupt_twice(Duration::from_secs(1));
    live.finish(SHORT).code(130).stderr_contains("pyfaro dev: stopped\n");
    wait_until_process_is_gone(pid, SHORT);
}

#[cfg(windows)]
#[test]
fn a_second_ctrl_c_forces_a_process_that_ignores_the_first_to_stop() {
    e2e_guard!();
    let sandbox = Sandbox::new();
    let root = module_project(&sandbox, SERVE);
    let mut live = Cmd::pyfaro(&root, ["dev", "--", "stubborn"]).spawn();
    live.wait_for_stdout("ready", STARTUP);
    let pid = pid_line(&live.stdout());
    live.interrupt_twice(Duration::from_secs(1));
    // The first Ctrl-C was sent a second earlier: waiting out its window
    // would take about four seconds more.
    let interrupted = Instant::now();
    let out = live.finish(SHORT);
    out.code(130).stderr_contains("pyfaro dev: stopped\n");
    assert!(
        interrupted.elapsed() < Duration::from_secs(2),
        "stopped after {:?}, expected at once",
        interrupted.elapsed()
    );
    wait_until_process_is_gone(pid, SHORT);
}

#[cfg(windows)]
#[test]
fn terminating_pyfaro_ends_a_module_process() {
    e2e_guard!();
    let sandbox = Sandbox::new();
    let root = module_project(&sandbox, SERVE);
    let mut live = Cmd::pyfaro(&root, ["dev"]).spawn();
    live.wait_for_stdout("ready", STARTUP);
    let pid = pid_line(&live.stdout());
    live.terminate();
    live.finish(SHORT);
    wait_until_process_is_gone(pid, SHORT);
}

// ---------------------------------------------------------------- FastAPI

#[test]
fn uvicorn_must_be_declared_for_the_fastapi_preset() {
    e2e_guard!();
    let sandbox = Sandbox::new();
    let root = sandbox.join("project");
    sandbox.write(
        "project/pyproject.toml",
        "[project]\nname = \"demo\"\nversion = \"0.1.0\"\n\
         dependencies = [\"fastapi\", \"uvicorn-worker\"]\n\n\
         [dependency-groups]\ndev = [\"pytest\"]\n\n\
         [tool.pyfaro.dev]\npreset = \"fastapi\"\nentry-point = \"demo.main:app\"\n",
    );
    let out = Cmd::pyfaro(&root, ["dev"]).run();
    out.code(1)
        .stdout_empty()
        .stderr_contains(
            "pyfaro dev: uvicorn is not declared in the project dependencies\n",
        )
        .stderr_contains("pyfaro add \"uvicorn[standard]\"");
    assert!(!root.join(".venv").exists(), "nothing may be launched");
}

#[test]
fn uvicorn_in_a_dependency_group_is_enough_and_its_status_is_forwarded() {
    e2e_guard!();
    let sandbox = Sandbox::new();
    let root = sandbox.join("project");
    sandbox.write(
        "project/pyproject.toml",
        "[project]\nname = \"demo\"\nversion = \"0.1.0\"\n\
         requires-python = \">=3.10\"\ndependencies = []\n\n\
         [dependency-groups]\ndev = [\"Uvicorn>=0.30\"]\n\n\
         [tool.pyfaro.dev]\npreset = \"fastapi\"\nentry-point = \"missing_app:app\"\n\
         reload = false\n",
    );
    let out =
        Cmd::pyfaro(&root, ["dev", "--", "--port", &free_port().to_string()])
            .run();
    // Uvicorn cannot import the app and exits with 3; its message is unmodified.
    out.code(3)
        .stderr_contains("Error loading ASGI app")
        .stderr_contains("pyfaro dev: uvicorn exited with status 3\n");
    assert!(!out.stderr.contains("not declared"), "{}", out.stderr);
}

#[test]
fn a_busy_port_is_reported_before_launch() {
    e2e_guard!();
    let sandbox = Sandbox::new();
    let root = sandbox.join("project");
    let holder = TcpListener::bind(("127.0.0.1", 0)).unwrap();
    let port = holder.local_addr().unwrap().port();
    sandbox.write(
        "project/pyproject.toml",
        "[project]\nname = \"demo\"\nversion = \"0.1.0\"\n\
         dependencies = [\"fastapi\", \"uvicorn\"]\n\n\
         [tool.pyfaro.dev]\npreset = \"fastapi\"\nentry-point = \"demo.main:app\"\n",
    );
    let guidance = [
        "stop the process using it",
        &format!("pyfaro dev -- --port {}", port + 1),
        "[tool.pyfaro.dev]",
    ];
    // The port comes from the command line ...
    let out =
        Cmd::pyfaro(&root, ["dev", "--", "--port", &port.to_string()]).run();
    out.code(1).stdout_empty().stderr_contains(&format!(
        "pyfaro dev: port {port} on 127.0.0.1 is already in use\n"
    ));
    for text in guidance {
        out.stderr_contains(text);
    }
    // ... with the equals form and an explicit host ...
    Cmd::pyfaro(
        &root,
        [
            "dev".to_owned(),
            "--".to_owned(),
            "--host=127.0.0.1".to_owned(),
            format!("--port={port}"),
        ],
    )
    .run()
    .code(1)
    .stderr_contains(&format!("port {port} on 127.0.0.1 is already in use"));
    assert!(!root.join(".venv").exists(), "nothing may be launched");
    // ... or from the configuration.
    let metadata =
        std::fs::read_to_string(root.join("pyproject.toml")).unwrap();
    sandbox.write(
        "project/pyproject.toml",
        &format!("{metadata}args = [\"--port\", \"{port}\"]\n"),
    );
    Cmd::pyfaro(&root, ["dev"]).run().code(1).stderr_contains(&format!(
        "port {port} on 127.0.0.1 is already in use"
    ));
    assert!(!root.join(".venv").exists(), "nothing may be launched");
}

#[test]
fn an_invalid_port_is_an_error_before_launch() {
    e2e_guard!();
    let sandbox = Sandbox::new();
    let root = sandbox.join("project");
    sandbox.write(
        "project/pyproject.toml",
        "[project]\nname = \"demo\"\nversion = \"0.1.0\"\n\
         dependencies = [\"uvicorn\"]\n\n\
         [tool.pyfaro.dev]\npreset = \"fastapi\"\nentry-point = \"demo.main:app\"\n",
    );
    for value in ["abc", "99999", "-1", ""] {
        let out = Cmd::pyfaro(&root, ["dev", "--", "--port", value]).run();
        out.code(1)
            .stdout_empty()
            .stderr_contains(&format!("pyfaro dev: invalid port {value:?}\n"));
    }
    assert!(!root.join(".venv").exists(), "nothing may be launched");
}

#[test]
fn fastapi_mode_serves_with_uvicorns_own_reload_and_stops_on_ctrl_c() {
    e2e_guard!();
    let sandbox = Sandbox::new();
    let port = free_port();
    let root = fastapi_project(
        &sandbox,
        "[tool.pyfaro.dev]\npreset = \"fastapi\"\nentry-point = \"demo.main:app\"\n",
    );
    let mut live = Cmd::pyfaro(
        &root,
        ["dev", "--verbose", "--", "--port", &port.to_string()],
    )
    .spawn();
    live.wait_for_port(port, STARTUP);
    live.wait_for_stderr("Application startup complete", SHORT);
    let (status, body) = live.http_get(port, "/");
    assert_eq!(status, 200, "{body}");
    assert!(body.contains("\"message\":\"hello\""), "{body}");
    // The app runs in the project root.
    assert!(body.contains("\"cwd\":\"project\""), "{body}");

    let stderr = live.stderr();
    assert!(
        stderr.contains(&format!(
            "pyfaro dev: starting demo.main:app with uvicorn (reload on) at http://127.0.0.1:{port} - press Ctrl-C to stop\n"
        )),
        "{stderr}"
    );
    let line = stderr
        .lines()
        .find(|line| line.starts_with("pyfaro dev: running "))
        .unwrap_or_else(|| panic!("no verbose line:\n{stderr}"));
    let separator = std::path::MAIN_SEPARATOR;
    assert!(
        line.contains(" -- uvicorn demo.main:app --reload --reload-dir "),
        "{line}"
    );
    assert!(
        line.ends_with(&format!("project{separator}src\" --port {port}")),
        "{line}"
    );
    // Uvicorn's own reloader runs; Pyfaro adds no watcher of its own.
    assert!(stderr.contains("Started reloader process"), "{stderr}");
    assert!(stderr.contains("Will watch for changes"), "{stderr}");
    // Server logs pass through unprefixed.
    assert!(
        stderr
            .lines()
            .any(|l| l.starts_with("INFO:") && l.contains("Uvicorn running")),
        "{stderr}"
    );

    stop_and_assert_clean(live, port, &root);
}

#[test]
fn reload_false_is_honored_and_extra_arguments_win() {
    e2e_guard!();
    let sandbox = Sandbox::new();
    // The configured port is taken; the command line overrides it.
    let holder = TcpListener::bind(("127.0.0.1", 0)).unwrap();
    let taken = holder.local_addr().unwrap().port();
    let port = free_port();
    let root = fastapi_project(
        &sandbox,
        &format!(
            "[tool.pyfaro.dev]\npreset = \"fastapi\"\nentry-point = \"demo.main:app\"\n\
             reload = false\nargs = [\"--port\", \"{taken}\"]\n"
        ),
    );
    let mut live =
        Cmd::pyfaro(&root, ["dev", "-v", "--", "--port", &port.to_string()])
            .spawn();
    live.wait_for_port(port, STARTUP);
    live.wait_for_stderr("Application startup complete", SHORT);
    let (status, body) = live.http_get(port, "/");
    assert_eq!(status, 200, "{body}");

    let stderr = live.stderr();
    assert!(
        stderr.contains(&format!(
            "pyfaro dev: starting demo.main:app with uvicorn (reload off) at http://127.0.0.1:{port} - "
        )),
        "{stderr}"
    );
    let line = stderr
        .lines()
        .find(|line| line.starts_with("pyfaro dev: running "))
        .unwrap();
    assert!(
        line.ends_with(&format!(
            " -- uvicorn demo.main:app --port {taken} --port {port}"
        )),
        "{line}"
    );
    assert!(!stderr.contains("Started reloader process"), "{stderr}");
    stop_and_assert_clean(live, port, &root);
}

// ------------------------------------------------- the FastAPI template

/// `pyfaro create fastapi` followed by `pyfaro sync`, in `<sandbox>/<name>`.
fn synced_fastapi_template(
    sandbox: &Sandbox,
    name: &str,
    package: Option<&str>,
) -> PathBuf {
    let mut create = vec!["create", "fastapi", name, "--python", "3.12"];
    if let Some(package) = package {
        create.extend(["--package", package]);
    }
    sandbox.pyfaro(create).run().success();
    let root = sandbox.join(name);
    Cmd::pyfaro(&root, ["sync"]).run().success();
    root
}

/// Processes whose command line mentions the sandbox: uv, Uvicorn's
/// reloader and its workers.
#[cfg(unix)]
fn sandbox_processes(sandbox: &Sandbox) -> Vec<u32> {
    let name = sandbox.path().file_name().unwrap().to_string_lossy();
    let unique = name.rsplit(' ').next().unwrap().to_owned();
    let listing = std::process::Command::new("pgrep")
        .args(["-f", &unique])
        .output()
        .expect("run pgrep");
    String::from_utf8_lossy(&listing.stdout)
        .lines()
        .filter_map(|line| line.trim().parse().ok())
        .collect()
}

/// Stop the template's server and check the port and the process tree.
fn stop_template(live: crate::harness::Live, port: u16, sandbox: &Sandbox) {
    // The process scan needs `pgrep`, so it only runs on Unix.
    #[cfg(not(unix))]
    let _ = sandbox;
    #[cfg(unix)]
    assert!(
        !sandbox_processes(sandbox).is_empty(),
        "the process scan finds nothing while the server runs"
    );
    let mut live = live;
    live.interrupt();
    live.finish(SHORT).code(130).stderr_contains("pyfaro dev: stopped\n");
    assert!(
        wait_until_port_is_free(port, SHORT),
        "port {port} is still bound after stopping"
    );
    #[cfg(unix)]
    for pid in sandbox_processes(sandbox) {
        wait_until_process_is_gone(pid, SHORT);
    }
}

#[test]
fn the_fastapi_template_is_served_by_pyfaro_dev() {
    e2e_guard!();
    let sandbox = Sandbox::new();
    let root = synced_fastapi_template(&sandbox, "acme-api", None);
    let port = free_port();
    let mut live =
        Cmd::pyfaro(&root, ["dev", "--", "--port", &port.to_string()]).spawn();
    live.wait_for_port(port, STARTUP);
    live.wait_for_stderr("Application startup complete", SHORT);
    let (status, body) = live.http_get(port, "/");
    assert_eq!(status, 200, "{body}");
    assert!(body.contains("Hello from acme-api!"), "{body}");
    let (status, body) = live.http_get(port, "/health");
    assert_eq!(status, 200, "{body}");
    assert!(body.contains("\"status\":\"ok\""), "{body}");
    assert!(
        live.stderr().contains(&format!(
            "pyfaro dev: starting acme_api.main:app with uvicorn (reload on) at http://127.0.0.1:{port} - press Ctrl-C to stop\n"
        )),
        "{}",
        live.stderr()
    );
    stop_template(live, port, &sandbox);
}

// On Windows, Uvicorn's reloader restarts its worker with a console Ctrl-C
// that reaches `pyfaro dev` too: it must keep running across the reloads. The
// process has a console of its own reached only through pipes, which holds
// that Ctrl-C back unless `pyfaro dev` touches the console (issue #129). Every
// step waits for what it caused, never for a fixed time.
#[test]
fn editing_the_template_reloads_and_a_syntax_error_is_visible() {
    e2e_guard!();
    let sandbox = Sandbox::new();
    let root = synced_fastapi_template(&sandbox, "acme-api", None);
    let port = free_port();
    let mut live =
        Cmd::pyfaro(&root, ["dev", "--", "--port", &port.to_string()]).spawn();
    live.wait_for_port(port, STARTUP);
    live.wait_for_stderr("Application startup complete", SHORT);
    live.wait_for_http(port, "/", "Hello from acme-api!", SHORT);

    // Uvicorn's own reloader picks up the edit.
    let main = root.join("src/acme_api/main.py");
    let original = std::fs::read_to_string(&main).unwrap();
    std::fs::write(
        &main,
        original.replace("Hello from acme-api!", "Reloaded by Uvicorn."),
    )
    .unwrap();
    live.wait_for_http(port, "/", "Reloaded by Uvicorn.", SHORT);
    assert!(live.is_running(), "pyfaro dev stopped: {}", live.stderr());

    // A syntax error shows its traceback; fixing it serves again.
    std::fs::write(&main, "def broken(:\n    pass\n").unwrap();
    live.wait_for_stderr("SyntaxError", SHORT);
    std::fs::write(
        &main,
        original.replace("Hello from acme-api!", "Fixed again."),
    )
    .unwrap();
    live.wait_for_http(port, "/", "Fixed again.", SHORT);
    assert!(live.is_running(), "pyfaro dev stopped: {}", live.stderr());

    stop_template(live, port, &sandbox);
}

#[test]
fn a_custom_package_template_is_served_by_pyfaro_dev() {
    e2e_guard!();
    let sandbox = Sandbox::new();
    let root = synced_fastapi_template(&sandbox, "acme-api", Some("backend"));
    let metadata =
        std::fs::read_to_string(root.join("pyproject.toml")).unwrap();
    assert!(
        metadata.contains("entry-point = \"backend.main:app\"\n"),
        "{metadata}"
    );
    let port = free_port();
    let mut live =
        Cmd::pyfaro(&root, ["dev", "--", "--port", &port.to_string()]).spawn();
    live.wait_for_port(port, STARTUP);
    live.wait_for_stderr("Application startup complete", SHORT);
    let (status, body) = live.http_get(port, "/");
    assert_eq!(status, 200, "{body}");
    assert!(body.contains("Hello from acme-api!"), "{body}");
    assert!(
        live.stderr()
            .contains("pyfaro dev: starting backend.main:app with uvicorn"),
        "{}",
        live.stderr()
    );
    stop_template(live, port, &sandbox);
}
