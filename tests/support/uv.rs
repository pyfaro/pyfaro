use std::{env, fs, process, thread, time::{Duration, Instant}};

fn main() {
    let args: Vec<_> = env::args().skip(1).collect();
    if args == ["--heartbeat"] {
        fs::write("uv-descendant.pid", process::id().to_string()).unwrap();
        let start = Instant::now();
        while start.elapsed() < Duration::from_secs(8) {
            fs::write("heartbeat", format!("{:?}", start.elapsed())).unwrap();
            thread::sleep(Duration::from_millis(20));
        }
        return;
    }
    let executable = env::current_exe().unwrap();
    let stem = executable.file_stem().unwrap().to_string_lossy();
    if (stem == "ty" && args == ["version"])
        || (stem != "ty" && args == ["--version"])
    {
        if env::var_os("PYFARO_FIXTURE_VERSION_FAIL").is_some() {
            eprintln!("fixture interpreter is broken");
            if env::var_os("PYFARO_FIXTURE_VERSION_MULTILINE").is_some() {
                eprintln!("second diagnostic line");
            }
            process::exit(7);
        }
        let default = match stem.as_ref() {
            "ruff" => "ruff 0.14.0",
            "ty" => "ty 0.0.1-alpha.24",
            "pytest" => "pytest 8.4.2",
            _ => "uv 0.9.26",
        };
        println!("{}", env::var("PYFARO_FIXTURE_VERSION")
            .unwrap_or_else(|_| default.into()));
        if env::var_os("PYFARO_FIXTURE_VERSION_EXTRA").is_some() {
            println!("plugin printed another line");
        }
        return;
    }
    if args.iter().any(|arg| arg == "record") {
        fs::write("uv-arguments.txt", args.join("\n")).unwrap();
        println!("fixture stdout");
        eprintln!("fixture stderr");
        process::exit(37);
    }
    let mode = env::var("PYFARO_FIXTURE_MODE").unwrap_or_default();
    if args.iter().any(|arg| arg == "lock") && mode == "fail-restore" {
        for file in ["pyproject.toml", "uv.lock"] {
            fs::remove_file(file).unwrap();
            fs::create_dir(file).unwrap();
        }
        process::exit(42);
    }
    if args.iter().any(|arg| arg == "lock") && (mode == "wait" || mode == "exit-with-child") {
        let mut child = process::Command::new(env::current_exe().unwrap())
            .arg("--heartbeat").spawn().unwrap();
        if mode == "exit-with-child" {
            let deadline = Instant::now() + Duration::from_secs(3);
            while !std::path::Path::new("heartbeat").exists() && Instant::now() < deadline {
                thread::sleep(Duration::from_millis(20));
            }
            process::exit(42);
        }
        fs::write("uv-ready", "ready").unwrap();
        let _ = child.wait();
        return;
    }
    if args.iter().any(|arg| arg == "init") && mode == "fail-init" {
        fs::write("pyproject.toml", "[project]\nname = 'partial'\n").unwrap();
        process::exit(23);
    }
    if args.iter().any(|arg| arg == "lock") && mode == "fail-lock" {
        fs::write("uv.lock", "incomplete lock\n").unwrap();
        eprintln!("fixture: cannot resolve dependencies");
        process::exit(42);
    }
    panic!("unexpected uv fixture arguments: {args:?}");
}
