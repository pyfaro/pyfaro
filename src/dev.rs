use std::ffi::OsString;
use std::io::{self, Write};
use std::net::TcpListener;
use std::path::Path;
use std::time::Duration;

use toml_edit::{DocumentMut, Item};

use crate::cli::DevArgs;
use crate::config::{Config, DevConfig, DevPreset};
use crate::error::{Error, Result};
use crate::quality::strip_verbatim;
use crate::{process, project, uv};

/// A server needs longer than a short-lived command to shut down cleanly.
const GRACE: Duration = Duration::from_secs(5);
const GUIDE: &str = "https://pyfaro.dev/commands/dev/";
const DEFAULT_HOST: &str = "127.0.0.1";
const DEFAULT_PORT: u16 = 8000;

pub(crate) fn run(args: DevArgs) -> Result<()> {
    let root = project::select(args.project.as_deref())?;
    let config = Config::load(&root)?;
    let Some(dev) = config.dev()? else {
        return Err(missing_table(&root));
    };
    if dev.preset.is_some() {
        let document = Config::load_document(&root)?;
        if !declares_requirement(&document, "uvicorn") {
            return Err(report(&[
                "uvicorn is not declared in the project dependencies".into(),
                "add it with: pyfaro add \"uvicorn[standard]\"".into(),
            ]));
        }
    }
    let plan = Plan::new(&dev, &args.extra, &root)?;
    if let Some(Listen::Tcp { host, port }) = &plan.listen {
        check_port(host, *port)?;
    }
    let cancelled = process::cancellation()?;
    let uv = uv::Uv::discover(args.uv, args.offline, &cancelled)?;
    let mut command = vec![
        OsString::from("run"),
        OsString::from("--project"),
        config.root().as_os_str().to_owned(),
    ];
    if args.locked {
        command.push(OsString::from("--locked"));
    } else if args.frozen {
        command.push(OsString::from("--frozen"));
    }
    command.push(OsString::from("--"));
    command.extend(plan.argv);
    say(&plan.startup);
    if args.verbose {
        say(&format!("running {}", uv.command_line(&command)));
    }
    match uv.execute_supervised(&root, "run", &command, GRACE, &cancelled) {
        Ok(()) => Ok(()),
        Err(Error::Cancelled) => {
            say("stopped");
            Err(Error::Reported { code: 130 })
        }
        Err(Error::UvStatus { status, .. }) => match status.code() {
            Some(code) => {
                say(&format!("{} exited with status {code}", plan.runner));
                Err(Error::Reported { code })
            }
            None => {
                say(&format!("{} was terminated ({status})", plan.runner));
                Err(Error::Reported { code: 1 })
            }
        },
        Err(error) => Err(error),
    }
}

/// What to launch for a `[tool.pyfaro.dev]` table plus extra arguments.
#[derive(Debug, PartialEq, Eq)]
struct Plan {
    /// The program named in diagnostics.
    runner: &'static str,
    /// The command after `uv run ... --`.
    argv: Vec<OsString>,
    startup: String,
    listen: Option<Listen>,
}

#[derive(Debug, PartialEq, Eq)]
enum Listen {
    Tcp {
        host: String,
        port: u16,
    },
    /// A Unix socket or an inherited descriptor: no port to check.
    Other,
}

impl Plan {
    fn new(dev: &DevConfig, extra: &[OsString], root: &Path) -> Result<Self> {
        let mut args: Vec<OsString> =
            dev.args.iter().map(OsString::from).collect();
        args.extend(extra.iter().cloned());
        let Some(DevPreset::FastApi) = dev.preset else {
            return Ok(Self {
                runner: "python",
                argv: [OsString::from("python"), OsString::from("-m")]
                    .into_iter()
                    .chain([OsString::from(&dev.entry_point)])
                    .chain(args)
                    .collect(),
                startup: format!(
                    "starting python -m {} (press Ctrl-C to stop)",
                    dev.entry_point
                ),
                listen: None,
            });
        };
        let words: Vec<String> =
            args.iter().map(|arg| arg.to_string_lossy().into_owned()).collect();
        let bind = Bind::scan(&words)?;
        let reload = dev.reload.unwrap_or(true);
        let mut argv =
            vec![OsString::from("uvicorn"), OsString::from(&dev.entry_point)];
        if reload {
            let source = root.join("src");
            let watched =
                if source.is_dir() { source } else { root.to_owned() };
            argv.push(OsString::from("--reload"));
            argv.push(OsString::from("--reload-dir"));
            argv.push(strip_verbatim(&watched).into_os_string());
        }
        argv.extend(args);
        let location = match &bind.location {
            Location::Tcp { host, port } => {
                let scheme = if bind.tls { "https" } else { "http" };
                format!("at {scheme}://{}:{port}", bracketed(host))
            }
            Location::Socket(path) => format!("on unix socket {path}"),
            Location::Descriptor(number) => {
                format!("on file descriptor {number}")
            }
        };
        let startup = format!(
            "starting {} with uvicorn (reload {}) {location} - press Ctrl-C to stop",
            dev.entry_point,
            if reload { "on" } else { "off" },
        );
        let listen = match bind.location {
            Location::Tcp { host, port } => Some(Listen::Tcp { host, port }),
            _ => Some(Listen::Other),
        };
        Ok(Self { runner: "uvicorn", argv, startup, listen })
    }
}

/// Where Uvicorn will listen, from the last `--host`/`--port` and friends.
struct Bind {
    location: Location,
    tls: bool,
}

enum Location {
    Tcp { host: String, port: u16 },
    Socket(String),
    Descriptor(String),
}

impl Bind {
    fn scan(args: &[String]) -> Result<Self> {
        let mut host = DEFAULT_HOST.to_owned();
        let mut port = DEFAULT_PORT;
        let mut socket = None;
        let mut descriptor = None;
        let mut tls = false;
        let mut index = 0;
        while index < args.len() {
            let (name, inline) = match args[index].split_once('=') {
                Some((name, value)) if name.starts_with("--") => {
                    (name, Some(value.to_owned()))
                }
                _ => (args[index].as_str(), None),
            };
            if matches!(
                name,
                "--host" | "--port" | "--uds" | "--fd" | "--ssl-keyfile"
            ) {
                let value = inline.or_else(|| {
                    index += 1;
                    args.get(index).cloned()
                });
                if let Some(value) = value {
                    match name {
                        "--host" => host = value,
                        "--port" => {
                            port = value.parse().map_err(|_| {
                                report(&[
                                    format!("invalid port {value:?}"),
                                    "--port takes an integer from 0 to 65535"
                                        .into(),
                                ])
                            })?;
                        }
                        "--uds" => socket = Some(value),
                        "--fd" => descriptor = Some(value),
                        _ => tls = true,
                    }
                }
            }
            index += 1;
        }
        let location = if let Some(path) = socket {
            Location::Socket(path)
        } else if let Some(number) = descriptor {
            Location::Descriptor(number)
        } else {
            Location::Tcp { host, port }
        };
        Ok(Self { location, tls })
    }
}

fn bracketed(host: &str) -> String {
    if host.contains(':') && !host.starts_with('[') {
        format!("[{host}]")
    } else {
        host.to_owned()
    }
}

/// Fail early, with guidance, when the port is taken. Other bind failures are
/// left for Uvicorn to report.
fn check_port(host: &str, port: u16) -> Result<()> {
    match TcpListener::bind((host, port)) {
        Err(error) if error.kind() == io::ErrorKind::AddrInUse => {
            let next = port.checked_add(1).unwrap_or(DEFAULT_PORT + 1);
            Err(report(&[
                format!("port {port} on {host} is already in use"),
                format!(
                    "stop the process using it, run pyfaro dev -- --port {next}, or set args = [\"--port\", \"{next}\"] in [tool.pyfaro.dev]"
                ),
            ]))
        }
        _ => Ok(()),
    }
}

fn missing_table(root: &Path) -> Error {
    let lines = [
        format!(
            "no [tool.pyfaro.dev] table in {}",
            root.join("pyproject.toml").display()
        ),
        "add one to run the project's development process, for example:".into(),
    ];
    let mut text = lines
        .iter()
        .map(|line| format!("pyfaro dev: {line}\n"))
        .collect::<String>();
    text.push_str(
        "\n    [tool.pyfaro.dev]\n    preset = \"fastapi\"\n    entry-point = \"my_package.main:app\"\n\n",
    );
    text.push_str("or, to run a module with python -m:\n");
    text.push_str(
        "\n    [tool.pyfaro.dev]\n    entry-point = \"my_package.worker\"\n\n",
    );
    text.push_str(&format!("pyfaro dev: see {GUIDE}\n"));
    let _ = io::stderr().lock().write_all(text.as_bytes());
    Error::Reported { code: 1 }
}

fn say(message: &str) {
    let _ = writeln!(io::stderr().lock(), "pyfaro dev: {message}");
}

/// Print `lines` as Pyfaro diagnostics; the caller exits with status 1.
fn report(lines: &[String]) -> Error {
    for line in lines {
        say(line);
    }
    Error::Reported { code: 1 }
}

/// Whether `[project].dependencies` or any dependency group declares `name`
/// (PEP 503 normalized, with extras, markers or version specifiers allowed).
fn declares_requirement(document: &DocumentMut, name: &str) -> bool {
    let project = document.get("project").and_then(Item::as_table_like);
    let dependencies = project
        .and_then(|project| project.get("dependencies"))
        .and_then(Item::as_array)
        .into_iter();
    let groups = document
        .get("dependency-groups")
        .and_then(Item::as_table_like)
        .into_iter()
        .flat_map(|groups| groups.iter())
        .filter_map(|(_, group)| group.as_array());
    dependencies
        .chain(groups)
        .flat_map(|array| array.iter())
        .filter_map(|requirement| requirement.as_str())
        .any(|requirement| requirement_name(requirement) == name)
}

fn requirement_name(requirement: &str) -> String {
    let mut name = String::new();
    let mut separator = false;
    for character in requirement.trim_start().chars().take_while(|c| {
        c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.')
    }) {
        if matches!(character, '-' | '_' | '.') {
            separator = true;
        } else {
            if separator {
                name.push('-');
                separator = false;
            }
            name.push(character.to_ascii_lowercase());
        }
    }
    name
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dev(preset: Option<DevPreset>, entry: &str) -> DevConfig {
        DevConfig {
            preset,
            entry_point: entry.to_owned(),
            args: Vec::new(),
            reload: None,
        }
    }

    fn os(args: &[&str]) -> Vec<OsString> {
        args.iter().map(OsString::from).collect()
    }

    fn words(argv: &[OsString]) -> Vec<String> {
        argv.iter().map(|arg| arg.to_string_lossy().into_owned()).collect()
    }

    #[test]
    fn requirement_names_are_normalized() {
        for (requirement, expected) in [
            ("uvicorn", "uvicorn"),
            ("Uvicorn[standard]>=0.30", "uvicorn"),
            ("  uvicorn ; python_version > '3.9'", "uvicorn"),
            ("uvi.corn_x==1", "uvi-corn-x"),
            ("uvicorn-worker", "uvicorn-worker"),
        ] {
            assert_eq!(requirement_name(requirement), expected);
        }
    }

    #[test]
    fn uvicorn_is_found_in_dependencies_and_groups() {
        let found = |source: &str| {
            declares_requirement(&source.parse().unwrap(), "uvicorn")
        };
        assert!(found("[project]\ndependencies = ['Uvicorn[standard]']\n"));
        assert!(found(
            "[project]\ndependencies = []\n[dependency-groups]\ndev = [{include-group = 'x'}, 'uvicorn>=1']\n"
        ));
        assert!(!found(
            "[project]\ndependencies = ['uvicorn-worker', 'fastapi']\n"
        ));
        assert!(!found("[project]\nname = 'x'\n"));
    }

    #[test]
    fn module_mode_runs_python_dash_m_with_args_in_order() {
        let mut config = dev(None, "demo.worker");
        config.args = vec!["--once".into()];
        let plan = Plan::new(&config, &os(&["x y"]), Path::new("/p")).unwrap();
        assert_eq!(plan.runner, "python");
        assert_eq!(
            words(&plan.argv),
            ["python", "-m", "demo.worker", "--once", "x y"]
        );
        assert_eq!(
            plan.startup,
            "starting python -m demo.worker (press Ctrl-C to stop)"
        );
        assert_eq!(plan.listen, None);
    }

    #[test]
    fn fastapi_defaults_to_reload_with_the_default_address() {
        let config = dev(Some(DevPreset::FastApi), "demo.main:app");
        let plan =
            Plan::new(&config, &[], Path::new("/does/not/exist")).unwrap();
        assert_eq!(
            words(&plan.argv),
            [
                "uvicorn",
                "demo.main:app",
                "--reload",
                "--reload-dir",
                "/does/not/exist"
            ]
        );
        assert_eq!(
            plan.startup,
            "starting demo.main:app with uvicorn (reload on) at http://127.0.0.1:8000 - press Ctrl-C to stop"
        );
        assert_eq!(
            plan.listen,
            Some(Listen::Tcp { host: "127.0.0.1".into(), port: 8000 })
        );
    }

    #[test]
    fn explicit_reload_false_wins_and_extra_args_come_last() {
        let mut config = dev(Some(DevPreset::FastApi), "demo.main:app");
        config.reload = Some(false);
        config.args = vec!["--port".into(), "8001".into()];
        let plan =
            Plan::new(&config, &os(&["--port=9000"]), Path::new("/p")).unwrap();
        assert_eq!(
            words(&plan.argv),
            ["uvicorn", "demo.main:app", "--port", "8001", "--port=9000"]
        );
        assert!(plan.startup.contains("(reload off) at http://127.0.0.1:9000"));
    }

    #[test]
    fn the_last_host_and_port_win_and_sockets_skip_the_check() {
        let scan = |args: &[&str]| {
            let words: Vec<String> =
                args.iter().map(|arg| arg.to_string()).collect();
            Bind::scan(&words)
        };
        let Ok(Bind { location: Location::Tcp { host, port }, tls }) = scan(&[
            "--host",
            "::1",
            "--port=1",
            "--host=0.0.0.0",
            "--port",
            "2",
        ]) else {
            panic!("expected a TCP bind");
        };
        assert_eq!((host.as_str(), port, tls), ("0.0.0.0", 2, false));
        assert!(matches!(
            scan(&["--uds", "/tmp/s"]),
            Ok(Bind { location: Location::Socket(_), .. })
        ));
        assert!(matches!(
            scan(&["--fd", "3", "--port", "9"]),
            Ok(Bind { location: Location::Descriptor(_), .. })
        ));
        assert!(matches!(
            scan(&["--ssl-keyfile", "k"]),
            Ok(Bind { tls: true, .. })
        ));
        for invalid in [["--port", "abc"], ["--port", "70000"], ["--port=", ""]]
        {
            assert!(scan(&invalid).is_err(), "accepted {invalid:?}");
        }
    }

    #[test]
    fn ipv6_hosts_are_bracketed_in_the_startup_url() {
        let config = dev(Some(DevPreset::FastApi), "a:app");
        let plan = Plan::new(&config, &os(&["--host", "::"]), Path::new("/p"))
            .unwrap();
        assert!(plan.startup.contains("at http://[::]:8000"));
    }

    #[test]
    fn module_args_keep_spaces_and_empty_values_verbatim() {
        let config = dev(None, "demo");
        let plan =
            Plan::new(&config, &os(&["a b", ""]), Path::new("/p")).unwrap();
        assert_eq!(words(&plan.argv)[3..], ["a b", ""]);
    }

    #[test]
    fn a_busy_port_is_reported_with_guidance() {
        let listener = TcpListener::bind(("127.0.0.1", 0)).unwrap();
        let port = listener.local_addr().unwrap().port();
        let error = check_port("127.0.0.1", port).unwrap_err();
        assert!(matches!(error, Error::Reported { code: 1 }));
        drop(listener);
        assert!(check_port("127.0.0.1", port).is_ok());
    }
}
