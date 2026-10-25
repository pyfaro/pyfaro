//! `pyfaro build`: wheels and source distributions through the real uv build,
//! validated by opening the artifacts with real Python and installing them.

use std::fs;
use std::path::{Path, PathBuf};
use std::thread;
use std::time::Duration;

use crate::harness::{Cmd, Kind, Outcome, Sandbox, UV};

const LIB_SDIST: &str = "acme_widget-0.1.0.tar.gz";
const LIB_WHEEL: &str = "acme_widget-0.1.0-py3-none-any.whl";

/// Reads one archive: `list` prints the member names, `read <suffix>` prints
/// the first member whose name ends with the suffix. Opening the archive
/// also checks it is intact.
const ARCHIVE: &str = r#"
import sys, tarfile, zipfile
path, mode = sys.argv[1], sys.argv[2]
if path.endswith(".whl"):
    with zipfile.ZipFile(path) as archive:
        assert archive.testzip() is None
        names = archive.namelist()
        read = lambda name: archive.read(name).decode("utf-8")
        if mode == "list":
            print("\n".join(names))
        else:
            print(read(next(n for n in names if n.endswith(sys.argv[3]))), end="")
else:
    with tarfile.open(path) as archive:
        names = archive.getnames()
        if mode == "list":
            print("\n".join(names))
        else:
            name = next(n for n in names if n.endswith(sys.argv[3]))
            print(archive.extractfile(name).read().decode("utf-8"), end="")
"#;

/// `<directory>/<file>` as `pyfaro build` prints it for a path below the
/// current directory.
fn shown(directory: &str, file: &str) -> String {
    let mut path: PathBuf = directory.split('/').collect();
    path.push(file);
    path.display().to_string()
}

fn built(lines: &[String]) -> String {
    lines.iter().map(|line| format!("built {line}\n")).collect()
}

/// Files of a directory, sorted.
fn names(directory: &Path) -> Vec<String> {
    let mut names: Vec<String> = fs::read_dir(directory)
        .unwrap_or_else(|error| panic!("{}: {error}", directory.display()))
        .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    names.sort();
    names
}

/// A fresh virtual environment `<sandbox>/<name>` on Python 3.12.
fn venv(sandbox: &Sandbox, name: &str) -> PathBuf {
    let path = sandbox.join(name);
    sandbox
        .uv(["venv", "--python", "3.12", "--no-progress"])
        .arg(&path)
        .run()
        .success();
    path
}

fn script(environment: &Path, name: &str) -> PathBuf {
    let directory = if cfg!(windows) { "Scripts" } else { "bin" };
    environment
        .join(directory)
        .join(format!("{name}{}", std::env::consts::EXE_SUFFIX))
}

/// The Python reading archives, run with `-I` so nothing is imported from
/// the working directory.
struct Reader {
    python: PathBuf,
    cwd: PathBuf,
}

impl Reader {
    fn new(sandbox: &Sandbox) -> Self {
        Self {
            python: script(&venv(sandbox, "reader"), "python"),
            cwd: sandbox.path().to_owned(),
        }
    }

    fn run(&self, archive: &Path, args: &[&str]) -> Outcome {
        Cmd::program(&self.python, &self.cwd, ["-I", "-c", ARCHIVE])
            .arg(archive)
            .args(args)
            .run()
    }

    /// Member names of an archive, one per line.
    fn list(&self, archive: &Path) -> String {
        let out = self.run(archive, &["list"]);
        out.success();
        out.stdout.replace('\r', "")
    }

    /// Text of the first member whose name ends with `suffix`.
    fn read(&self, archive: &Path, suffix: &str) -> String {
        let out = self.run(archive, &["read", suffix]);
        out.success();
        out.stdout.replace('\r', "")
    }
}

fn assert_has(listing: &str, member: &str) {
    assert!(
        listing.lines().any(|line| line == member),
        "{member} missing from:\n{listing}"
    );
}

/// Check a wheel and an sdist of `name` 0.1.0 with the package `package`.
fn assert_artifacts(
    reader: &Reader,
    dist: &Path,
    name: &str,
    package: &str,
    generator: &str,
) {
    let stem = package;
    let wheel = dist.join(format!("{stem}-0.1.0-py3-none-any.whl"));
    let sdist = dist.join(format!("{stem}-0.1.0.tar.gz"));

    let listing = reader.list(&wheel);
    assert_has(&listing, &format!("{package}/__init__.py"));
    assert_has(&listing, &format!("{package}/py.typed"));
    assert_has(&listing, &format!("{stem}-0.1.0.dist-info/METADATA"));
    let metadata = reader.read(&wheel, ".dist-info/METADATA");
    assert!(metadata.contains(&format!("\nName: {name}\n")), "{metadata}");
    assert!(metadata.contains("\nVersion: 0.1.0\n"), "{metadata}");
    let tags = reader.read(&wheel, ".dist-info/WHEEL");
    assert!(
        tags.lines().any(|line| line.starts_with(generator)),
        "expected a {generator:?} line in:\n{tags}"
    );

    let listing = reader.list(&sdist);
    assert_has(&listing, &format!("{stem}-0.1.0/PKG-INFO"));
    assert_has(&listing, &format!("{stem}-0.1.0/pyproject.toml"));
    assert_has(&listing, &format!("{stem}-0.1.0/src/{package}/__init__.py"));
    let info = reader.read(&sdist, "/PKG-INFO");
    assert!(info.contains(&format!("\nName: {name}\n")), "{info}");
    assert!(info.contains("\nVersion: 0.1.0\n"), "{info}");
}

#[test]
fn builds_a_library_into_dist() {
    e2e_guard!();
    let sandbox = Sandbox::new();
    let project = sandbox.project(Kind::Lib);
    let metadata = project.read("pyproject.toml");
    project
        .pyfaro(["build"])
        .run()
        .success()
        .stdout_eq(&built(&[
            shown("dist", LIB_SDIST),
            shown("dist", LIB_WHEEL),
        ]))
        .stderr_contains("Successfully built");
    assert_eq!(
        names(&project.path("dist")),
        [".gitignore", LIB_WHEEL, LIB_SDIST]
    );
    let reader = Reader::new(&sandbox);
    assert_artifacts(
        &reader,
        &project.path("dist"),
        "acme-widget",
        "acme_widget",
        "Generator: uv",
    );
    // Pyfaro does not touch the project: no edits, no lock, no environment.
    assert_eq!(project.read("pyproject.toml"), metadata);
    assert!(!project.path("uv.lock").exists());
    assert!(!project.path(".venv").exists());
}

#[test]
fn builds_a_cli_whose_wheel_installs_and_runs() {
    e2e_guard!();
    let sandbox = Sandbox::new();
    let project = sandbox.project(Kind::Cli);
    project.pyfaro(["build"]).run().success().stdout_eq(&built(&[
        shown("dist", "acme_tools-0.1.0.tar.gz"),
        shown("dist", "acme_tools-0.1.0-py3-none-any.whl"),
    ]));
    let dist = project.path("dist");
    let reader = Reader::new(&sandbox);
    assert_artifacts(
        &reader,
        &dist,
        "acme-tools",
        "acme_tools",
        "Generator: uv",
    );
    let wheel = dist.join("acme_tools-0.1.0-py3-none-any.whl");
    let entry_points = reader.read(&wheel, ".dist-info/entry_points.txt");
    assert!(
        entry_points.contains("acme-tools = acme_tools.cli:main"),
        "{entry_points}"
    );

    // Install the built wheel, not the source tree, into a fresh environment.
    let environment = venv(&sandbox, "consumer");
    sandbox
        .uv(["pip", "install", "--no-progress", "--python"])
        .arg(&environment)
        .arg(&wheel)
        .run()
        .success();
    Cmd::program(script(&environment, "acme-tools"), sandbox.path(), [""; 0])
        .run()
        .success()
        .stdout_text_eq("Hello from acme-tools!\n");
    Cmd::program(
        script(&environment, "python"),
        sandbox.path(),
        ["-I", "-c", "import acme_tools; print(acme_tools.__file__)"],
    )
    .run()
    .success()
    .stdout_contains("site-packages");

    // The sdist builds a wheel as well: that is how `pyfaro build` works.
    let from_sdist = sandbox.join("from-sdist");
    sandbox
        .uv(["build", "--no-progress", "--wheel", "--out-dir"])
        .arg(&from_sdist)
        .arg(dist.join("acme_tools-0.1.0.tar.gz"))
        .run()
        .success();
    assert_eq!(
        names(&from_sdist),
        [".gitignore", "acme_tools-0.1.0-py3-none-any.whl"]
    );
}

#[test]
fn sdist_and_wheel_flags_select_the_kinds() {
    e2e_guard!();
    let sandbox = Sandbox::new();
    let project = sandbox.project(Kind::Lib);
    project
        .pyfaro(["build", "--sdist"])
        .run()
        .success()
        .stdout_eq(&built(&[shown("dist", LIB_SDIST)]));
    assert_eq!(names(&project.path("dist")), [".gitignore", LIB_SDIST]);

    // The sdist from the first run is not rewritten, so it is not reported.
    project
        .pyfaro(["build", "--wheel"])
        .run()
        .success()
        .stdout_eq(&built(&[shown("dist", LIB_WHEEL)]));
    assert_eq!(
        names(&project.path("dist")),
        [".gitignore", LIB_WHEEL, LIB_SDIST]
    );

    // Both flags are the default: a source distribution, then a wheel.
    project
        .pyfaro(["build", "--wheel", "--sdist", "--clear"])
        .run()
        .success()
        .stdout_eq(&built(&[
            shown("dist", LIB_SDIST),
            shown("dist", LIB_WHEEL),
        ]));
    let reader = Reader::new(&sandbox);
    assert_artifacts(
        &reader,
        &project.path("dist"),
        "acme-widget",
        "acme_widget",
        "Generator: uv",
    );
}

#[test]
fn a_wheel_only_build_validates_without_an_sdist() {
    e2e_guard!();
    let sandbox = Sandbox::new();
    let project = sandbox.project(Kind::Lib);
    project
        .pyfaro(["build", "--wheel"])
        .run()
        .success()
        .stdout_eq(&built(&[shown("dist", LIB_WHEEL)]));
    let reader = Reader::new(&sandbox);
    let wheel = project.path("dist").join(LIB_WHEEL);
    assert_has(&reader.list(&wheel), "acme_widget/__init__.py");
    let metadata = reader.read(&wheel, ".dist-info/METADATA");
    assert!(metadata.contains("\nName: acme-widget\n"), "{metadata}");
}

#[test]
fn out_dir_is_relative_to_the_current_directory() {
    e2e_guard!();
    let sandbox = Sandbox::new();
    let project = sandbox.project(Kind::Lib);
    // Run from the sandbox, not the project: `artifacts` is below the
    // sandbox, and `dist` is not created in the project.
    sandbox
        .pyfaro(["build", "--project", "project", "--out-dir", "artifacts"])
        .run()
        .success()
        .stdout_eq(&built(&[
            shown("artifacts", LIB_SDIST),
            shown("artifacts", LIB_WHEEL),
        ]));
    assert_eq!(
        names(&sandbox.join("artifacts")),
        [".gitignore", LIB_WHEEL, LIB_SDIST]
    );
    assert!(!project.path("dist").exists());

    // An absolute directory is used as given; the printed path is relative
    // to the current directory, here the project.
    let absolute = sandbox.join("absolute out");
    let out =
        project.pyfaro(["build", "--wheel", "--out-dir"]).arg(&absolute).run();
    out.success();
    assert_eq!(names(&absolute), [".gitignore", LIB_WHEEL]);
    out.stdout_eq(&built(&[Path::new("..")
        .join("absolute out")
        .join(LIB_WHEEL)
        .display()
        .to_string()]));
}

#[test]
fn artifacts_are_shown_relative_to_the_current_directory() {
    e2e_guard!();
    let sandbox = Sandbox::new();
    let project = sandbox.project(Kind::Lib);
    // Discovered from a subdirectory, the default `dist` is above it.
    let up = Path::new("..").join("dist");
    project
        .pyfaro(["build"])
        .cwd(project.path("src"))
        .run()
        .success()
        .stdout_eq(&built(&[
            up.join(LIB_SDIST).display().to_string(),
            up.join(LIB_WHEEL).display().to_string(),
        ]));
    assert!(project.path("dist").join(LIB_WHEEL).is_file());
}

#[test]
fn clear_removes_stale_artifacts() {
    e2e_guard!();
    let sandbox = Sandbox::new();
    let project = sandbox.project(Kind::Lib);
    project.write("dist/stale.txt", "stale\n");
    project.write("dist/old_widget-0.0.1.tar.gz", "not an archive\n");
    project.pyfaro(["build"]).run().success().stdout_eq(&built(&[
        shown("dist", LIB_SDIST),
        shown("dist", LIB_WHEEL),
    ]));
    // Without --clear everything stays.
    assert_eq!(
        names(&project.path("dist")),
        [
            ".gitignore",
            LIB_WHEEL,
            LIB_SDIST,
            "old_widget-0.0.1.tar.gz",
            "stale.txt"
        ]
    );

    project.pyfaro(["build", "--clear"]).run().success().stdout_eq(&built(&[
        shown("dist", LIB_SDIST),
        shown("dist", LIB_WHEEL),
    ]));
    assert_eq!(
        names(&project.path("dist")),
        [".gitignore", LIB_WHEEL, LIB_SDIST]
    );
}

#[test]
fn clear_refuses_to_empty_the_project() {
    e2e_guard!();
    let sandbox = Sandbox::new();
    let project = sandbox.project(Kind::Lib);
    let before = project.snapshot();
    for out_dir in [".", ".."] {
        project
            .pyfaro(["build", "--clear", "--out-dir", out_dir])
            .run()
            .code(2)
            .stdout_empty()
            .stderr_contains("error: --clear would delete the project")
            .stderr_contains("choose another --out-dir");
        assert_eq!(project.snapshot(), before);
    }
    // The same directory is fine without --clear.
    project
        .pyfaro(["build", "--wheel", "--out-dir", "."])
        .run()
        .success()
        .stdout_eq(&built(&[LIB_WHEEL.to_owned()]));
    assert!(project.path(LIB_WHEEL).is_file());
    assert!(project.path("pyproject.toml").is_file());
}

#[test]
fn a_rerun_overwrites_and_reports_the_artifacts_again() {
    e2e_guard!();
    let sandbox = Sandbox::new();
    let project = sandbox.project(Kind::Lib);
    project.pyfaro(["build"]).run().success();
    let modified = |file: &str| {
        fs::metadata(project.path("dist").join(file))
            .unwrap()
            .modified()
            .unwrap()
    };
    let (sdist, wheel) = (modified(LIB_SDIST), modified(LIB_WHEEL));
    thread::sleep(Duration::from_millis(100));
    project.pyfaro(["build"]).run().success().stdout_eq(&built(&[
        shown("dist", LIB_SDIST),
        shown("dist", LIB_WHEEL),
    ]));
    assert!(modified(LIB_SDIST) > sdist);
    assert!(modified(LIB_WHEEL) > wheel);
    let reader = Reader::new(&sandbox);
    assert_artifacts(
        &reader,
        &project.path("dist"),
        "acme-widget",
        "acme_widget",
        "Generator: uv",
    );

    // A new version adds artifacts; only those are reported.
    let metadata = project.read("pyproject.toml");
    project.write(
        "pyproject.toml",
        &metadata.replace("version = \"0.1.0\"", "version = \"0.2.0\""),
    );
    project.pyfaro(["build"]).run().success().stdout_eq(&built(&[
        shown("dist", "acme_widget-0.2.0.tar.gz"),
        shown("dist", "acme_widget-0.2.0-py3-none-any.whl"),
    ]));
    assert_eq!(names(&project.path("dist")).len(), 5);
}

#[test]
fn offline_builds_with_the_uv_build_backend() {
    e2e_guard!();
    let sandbox = Sandbox::new();
    let project = sandbox.project(Kind::Lib);
    project
        .pyfaro(["build", "--offline", "--wheel"])
        .env("UV_CACHE_DIR", sandbox.join("empty cache"))
        .run()
        .success()
        .stdout_eq(&built(&[shown("dist", LIB_WHEEL)]));
}

/// A project that declares `build_system` (possibly nothing) in the sandbox.
fn custom_project(
    sandbox: &Sandbox,
    name: &str,
    build_system: &str,
    extra: &str,
) -> PathBuf {
    let package = "acme_custom";
    sandbox.write(
        "custom/pyproject.toml",
        &format!(
            "[project]\nname = \"{name}\"\nversion = \"0.1.0\"\n\
             requires-python = \">=3.10\"\n{build_system}{extra}"
        ),
    );
    sandbox.write(format!("custom/src/{package}/__init__.py"), "VALUE = 1\n");
    sandbox.write(format!("custom/src/{package}/py.typed"), "");
    sandbox.join("custom")
}

fn assert_custom(sandbox: &Sandbox, root: &Path, generator: &str) {
    let dist = root.join("dist");
    let sdist = "acme_custom-0.1.0.tar.gz";
    let wheel = "acme_custom-0.1.0-py3-none-any.whl";
    let reader = Reader::new(sandbox);
    let listing = reader.list(&dist.join(wheel));
    assert_has(&listing, "acme_custom/__init__.py");
    let tags = reader.read(&dist.join(wheel), ".dist-info/WHEEL");
    assert!(
        tags.lines().any(|line| line.starts_with(generator)),
        "expected a {generator:?} line in:\n{tags}"
    );
    let metadata = reader.read(&dist.join(wheel), ".dist-info/METADATA");
    assert!(metadata.contains("\nName: Acme.Custom\n"), "{metadata}");
    assert!(metadata.contains("\nVersion: 0.1.0\n"), "{metadata}");
    let info = reader.read(&dist.join(sdist), "/PKG-INFO");
    assert!(info.contains("\nVersion: 0.1.0\n"), "{info}");
    assert_has(
        &reader.list(&dist.join(sdist)),
        "acme_custom-0.1.0/src/acme_custom/__init__.py",
    );
}

#[test]
fn builds_with_the_declared_hatchling_backend() {
    e2e_guard!();
    let sandbox = Sandbox::new();
    let root = custom_project(
        &sandbox,
        "Acme.Custom",
        "\n[build-system]\nrequires = [\"hatchling\"]\n\
         build-backend = \"hatchling.build\"\n",
        "\n[tool.hatch.build.targets.wheel]\npackages = [\"src/acme_custom\"]\n",
    );
    let metadata = fs::read(root.join("pyproject.toml")).unwrap();
    sandbox.pyfaro(["build", "--project", "custom"]).run().success().stdout_eq(
        &built(&[
            shown("custom/dist", "acme_custom-0.1.0.tar.gz"),
            shown("custom/dist", "acme_custom-0.1.0-py3-none-any.whl"),
        ]),
    );
    assert_custom(&sandbox, &root, "Generator: hatchling");
    assert_eq!(fs::read(root.join("pyproject.toml")).unwrap(), metadata);
}

#[test]
fn builds_with_the_declared_setuptools_backend() {
    e2e_guard!();
    let sandbox = Sandbox::new();
    let root = custom_project(
        &sandbox,
        "Acme.Custom",
        "\n[build-system]\nrequires = [\"setuptools>=68\"]\n\
         build-backend = \"setuptools.build_meta\"\n",
        "",
    );
    let metadata = fs::read(root.join("pyproject.toml")).unwrap();
    sandbox.pyfaro(["build", "--project", "custom"]).run().success().stdout_eq(
        &built(&[
            shown("custom/dist", "acme_custom-0.1.0.tar.gz"),
            shown("custom/dist", "acme_custom-0.1.0-py3-none-any.whl"),
        ]),
    );
    assert_custom(&sandbox, &root, "Generator: setuptools");
    assert_eq!(fs::read(root.join("pyproject.toml")).unwrap(), metadata);
}

#[test]
fn builds_with_uv_default_backend_when_no_build_system_is_declared() {
    e2e_guard!();
    let sandbox = Sandbox::new();
    let root = custom_project(&sandbox, "Acme.Custom", "", "");
    sandbox.pyfaro(["build", "--project", "custom"]).run().success().stdout_eq(
        &built(&[
            shown("custom/dist", "acme_custom-0.1.0.tar.gz"),
            shown("custom/dist", "acme_custom-0.1.0-py3-none-any.whl"),
        ]),
    );
    assert_custom(&sandbox, &root, "Generator: setuptools");
    assert!(
        !fs::read_to_string(root.join("pyproject.toml"))
            .unwrap()
            .contains("build-system")
    );
}

#[test]
fn a_broken_build_forwards_uvs_exit_status_and_error() {
    e2e_guard!();
    let sandbox = Sandbox::new();
    let project = sandbox.project(Kind::Lib);
    fs::remove_dir_all(project.path("src/acme_widget")).unwrap();
    let out = project.pyfaro(["build"]).run();
    let uv = sandbox
        .uv(["build", "--no-progress", "--out-dir"])
        .arg(sandbox.join("uv-out"))
        .arg(&project.root)
        .run();
    let code = uv.code.expect("uv exit code");
    assert_ne!(code, 0);
    out.code(code)
        .stdout_empty()
        .stderr_contains("Expected a Python module at")
        .stderr_contains("error: uv build failed (exit ")
        // "exit status: N" on Unix, "exit code: N" on Windows.
        .stderr_contains(&format!(": {code}); see uv's output above"));
    assert!(!project.path("dist").join(LIB_WHEEL).exists());
}

#[test]
fn offline_builds_fail_when_the_backend_is_not_cached() {
    e2e_guard!();
    let sandbox = Sandbox::new();
    let root = custom_project(
        &sandbox,
        "Acme.Custom",
        "\n[build-system]\nrequires = [\"setuptools>=68\"]\n\
         build-backend = \"setuptools.build_meta\"\n",
        "",
    );
    let out = sandbox
        .pyfaro(["build", "--offline", "--project", "custom"])
        .env("UV_CACHE_DIR", sandbox.join("empty cache"))
        .run();
    assert_ne!(out.code, Some(0));
    assert_ne!(out.code, None);
    out.stdout_empty()
        .stderr_contains("the network was disabled")
        .stderr_contains("error: uv build failed (");
    assert!(!root.join("dist").join("acme_custom-0.1.0.tar.gz").exists());
}

#[test]
fn reports_a_missing_project() {
    e2e_guard!();
    let sandbox = Sandbox::new();
    sandbox
        .pyfaro(["build"])
        .run()
        .code(1)
        .stdout_empty()
        .stderr_contains("error: no pyproject.toml found from")
        .stderr_contains("use --project <PATH>");
    fs::create_dir(sandbox.join("empty")).unwrap();
    sandbox
        .pyfaro(["build", "--project", "empty"])
        .run()
        .code(1)
        .stdout_empty()
        .stderr_contains("error: project directory has no pyproject.toml");
    assert!(!sandbox.join("dist").exists());
    assert!(!sandbox.join("empty").join("dist").exists());
}

#[test]
fn reports_a_missing_uv_with_the_remedy() {
    e2e_guard!();
    let sandbox = Sandbox::new();
    let project = sandbox.project(Kind::Lib);
    let before = project.snapshot();
    project
        .pyfaro(["build", "--uv", "pyfaro-e2e-no-such-uv"])
        .run()
        .code(1)
        .stdout_empty()
        .stderr_contains("error: could not find uv at pyfaro-e2e-no-such-uv")
        .stderr_contains("install uv or use --uv <path>");
    assert_eq!(project.snapshot(), before);
    assert!(!project.path("dist").exists());
}

/// A shell script that answers `--version` like uv 0.9.26 and, for `build`,
/// writes the artifacts named in `files` into the `--out-dir` it is given.
#[cfg(unix)]
fn fake_uv(sandbox: &Sandbox, files: &[&str]) -> PathBuf {
    use std::os::unix::fs::PermissionsExt;

    let writes: String = files
        .iter()
        .map(|file| format!("echo fake > \"$out/{file}\"\n"))
        .collect();
    let uv = sandbox.write(
        "fake-uv",
        &format!(
            "#!/bin/sh\n\
             case \"$1\" in --version) echo 'uv 0.9.26 (fake)'; exit 0;; esac\n\
             out=\n\
             while [ $# -gt 0 ]; do\n\
             if [ \"$1\" = --out-dir ]; then out=$2; fi\n\
             shift\n\
             done\n\
             mkdir -p \"$out\"\n{writes}exit 0\n"
        ),
    );
    fs::set_permissions(&uv, fs::Permissions::from_mode(0o755)).unwrap();
    uv
}

#[cfg(unix)]
#[test]
fn fails_when_uv_succeeds_without_writing_the_expected_artifacts() {
    e2e_guard!();
    let sandbox = Sandbox::new();
    let project = sandbox.project(Kind::Lib);

    let nothing = fake_uv(&sandbox, &[]);
    project
        .pyfaro(["build", "--uv"])
        .arg(&nothing)
        .run()
        .code(1)
        .stdout_empty()
        .stderr_contains("error: uv build succeeded but wrote no source distribution (.tar.gz) for acme_widget in ")
        .stderr_contains("check the build backend in [build-system] of pyproject.toml");

    // An artifact of another project does not count.
    let other =
        fake_uv(&sandbox, &["other-1.0.tar.gz", "other-1.0-py3-none-any.whl"]);
    project
        .pyfaro(["build", "--uv"])
        .arg(&other)
        .run()
        .code(1)
        .stdout_empty()
        .stderr_contains(
            "wrote no source distribution (.tar.gz) for acme_widget",
        );

    let sdist_only = fake_uv(&sandbox, &[LIB_SDIST]);
    project
        .pyfaro(["build", "--uv"])
        .arg(&sdist_only)
        .run()
        .code(1)
        .stdout_empty()
        .stderr_contains("wrote no wheel (.whl) for acme_widget in ");
    // The same output satisfies an sdist-only build.
    project
        .pyfaro(["build", "--sdist", "--uv"])
        .arg(&sdist_only)
        .run()
        .success();
    assert!(project.path("dist").join(LIB_SDIST).is_file());
}

#[test]
fn verbose_shows_the_uv_build_command_line() {
    e2e_guard!();
    let sandbox = Sandbox::new();
    let project = sandbox.project(Kind::Lib);
    for args in [
        ["-v", "build", "--wheel", "--clear"],
        ["build", "--verbose", "--wheel", "--clear"],
    ] {
        let out = project.pyfaro(args).run();
        out.success();
        // The artifact lines stay on stdout, the trace goes to stderr.
        out.stdout_eq(&built(&[shown("dist", LIB_WHEEL)]));
        let lines: Vec<&str> = out
            .stderr
            .lines()
            .filter(|line| line.starts_with("pyfaro build: "))
            .collect();
        let using = format!("pyfaro build: using uv {UV} ");
        assert!(
            lines.iter().any(|line| line.starts_with(&using)),
            "{lines:#?}"
        );
        let running = lines
            .iter()
            .find(|line| line.contains("--no-python-downloads"))
            .unwrap_or_else(|| panic!("no uv command line in {lines:#?}"));
        assert!(
            running.starts_with("pyfaro build: running ")
                && running.contains(
                    " --no-python-downloads --no-progress build --wheel --clear --out-dir "
                ),
            "{running}"
        );
        // The project root is passed explicitly, as the last argument.
        let root = fs::canonicalize(&project.root).unwrap();
        assert!(
            running
                .trim_end_matches('"')
                .ends_with(&root.display().to_string()),
            "{running}"
        );
    }
    // Without the flag Pyfaro prints nothing of its own.
    let quiet = project.pyfaro(["build", "--wheel"]).run();
    quiet.success();
    assert!(!quiet.stderr.contains("pyfaro build:"), "{}", quiet.stderr);
}
