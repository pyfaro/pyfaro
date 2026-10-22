//! `pyfaro lint`, `fmt`, `type`, `check` and `test` against generated, synced
//! projects with the real, pinned Ruff, ty and pytest.

use crate::harness::{Cmd, Kind, Outcome, Project, Sandbox};

const UNUSED_IMPORT: &str = "import os\n";
const UNFORMATTED: &str = "VALUE=1\n";
const TYPE_ERROR: &str = "VALUE: int = \"text\"\n";
const FAILING_TEST: &str =
    "def test_failure() -> None:\n    assert 1 + 1 == 3\n";

/// The message for a tool that cannot be found.
fn missing(tool: &str) -> String {
    format!(
        "{tool}: not found in the project environment, VIRTUAL_ENV, or PATH; run `pyfaro add --dev {tool}` then `pyfaro sync`"
    )
}

/// The reason a summary line gives for a tool that cannot be found.
fn unavailable(tool: &str) -> String {
    missing(tool).replacen(": ", " ", 1)
}

/// The summary block `pyfaro check` prints, from its step lines.
fn summary(lines: &[&str]) -> String {
    format!("\n== pyfaro check summary ==\n{}\n", lines.join("\n"))
}

/// Everything a run printed; tools differ in which stream they use.
fn printed(out: &Outcome) -> String {
    format!("{}{}", out.stdout, out.stderr)
}

fn assert_printed(out: &Outcome, needle: &str) {
    let text = printed(out);
    assert!(text.contains(needle), "output lacks {needle:?}:\n{text}");
}

fn assert_not_printed(out: &Outcome, needle: &str) {
    let text = printed(out);
    assert!(!text.contains(needle), "output has {needle:?}:\n{text}");
}

/// Run `body` on a synced library project and on a synced CLI project.
fn for_each_kind(body: impl Fn(&Project)) {
    for kind in [Kind::Lib, Kind::Cli] {
        let sandbox = Sandbox::new();
        body(&sandbox.synced_project(kind));
    }
}

/// Remove a package from the project's `.venv`, so its tool is missing.
fn uninstall(project: &Project, package: &str) {
    project
        .uv(["pip", "uninstall", "--python"])
        .arg(project.path(".venv"))
        .arg(package)
        .run()
        .success();
}

/// Keep a tool from being found on `PATH`, so only the project environment
/// can provide it.
fn empty_path(cmd: Cmd, sandbox: &Sandbox) -> Cmd {
    cmd.env("PATH", sandbox.join("no-tools"))
}

/// Exclude a file from Ruff and ty through the project configuration.
fn exclude(project: &Project, relative: &str) {
    let text = project.read("pyproject.toml").replace(
        "line-length = 88\n",
        &format!("line-length = 88\nextend-exclude = [\"{relative}\"]\n"),
    );
    let text = format!("{text}\n[tool.ty.src]\nexclude = [\"{relative}\"]\n");
    project.write("pyproject.toml", &text);
}

/// A project with an excluded file that violates lint, format and types,
/// returning the excluded file's project-relative path.
fn with_excluded_file(project: &Project) -> String {
    let relative = format!("src/{}/excluded.py", project.package);
    project.write(&relative, "import os\nVALUE: int = \"x\"\nOTHER=2\n");
    exclude(project, &relative);
    relative
}

// ---------------------------------------------------------------- all kinds

#[test]
fn every_command_succeeds_on_a_fresh_project() {
    e2e_guard!();
    for_each_kind(|project| {
        for args in [
            vec!["check"],
            vec!["lint"],
            vec!["fmt", "--check"],
            vec!["type"],
            vec!["test"],
        ] {
            project.pyfaro(&args).run().success();
        }
    });
}

#[test]
fn a_lint_violation_fails_only_lint_and_check() {
    e2e_guard!();
    for_each_kind(|project| {
        project
            .write(format!("src/{}/extra.py", project.package), UNUSED_IMPORT);
        let lint = project.pyfaro(["lint"]).run();
        lint.code(1).stderr_contains("error: ruff failed (exit");
        assert_printed(&lint, "F401");
        project.pyfaro(["check"]).run().code(1);
        project.pyfaro(["fmt", "--check"]).run().success();
        project.pyfaro(["type"]).run().success();
        project.pyfaro(["test"]).run().success();
    });
}

#[test]
fn a_format_issue_fails_only_fmt_check_and_check() {
    e2e_guard!();
    for_each_kind(|project| {
        project.write(format!("src/{}/extra.py", project.package), UNFORMATTED);
        project
            .pyfaro(["fmt", "--check"])
            .run()
            .code(1)
            .stderr_contains("error: ruff failed (exit");
        project.pyfaro(["check"]).run().code(1);
        project.pyfaro(["lint"]).run().success();
        project.pyfaro(["type"]).run().success();
        project.pyfaro(["test"]).run().success();
    });
}

#[test]
fn a_type_error_fails_only_type_and_check() {
    e2e_guard!();
    for_each_kind(|project| {
        project.write(format!("src/{}/extra.py", project.package), TYPE_ERROR);
        project.pyfaro(["type"]).run().code(1);
        project.pyfaro(["check"]).run().code(1);
        project.pyfaro(["lint"]).run().success();
        project.pyfaro(["fmt", "--check"]).run().success();
        project.pyfaro(["test"]).run().success();
    });
}

#[test]
fn a_failing_test_fails_only_test() {
    e2e_guard!();
    for_each_kind(|project| {
        project.write("tests/test_failure.py", FAILING_TEST);
        project.pyfaro(["test"]).run().code(1);
        for args in
            [vec!["check"], vec!["lint"], vec!["type"], vec!["fmt", "--check"]]
        {
            project.pyfaro(&args).run().success();
        }
    });
}

#[test]
fn excluded_files_stay_excluded_even_when_named() {
    e2e_guard!();
    for_each_kind(|project| {
        let relative = with_excluded_file(project);
        project.pyfaro(["lint"]).run().success();
        project.pyfaro(["lint", &relative]).run().success();
        project.pyfaro(["fmt", "--check", &relative]).run().success();
        project.pyfaro(["type"]).run().success();
        // ty ignores the exclusion for a named file on Windows (#109).
        #[cfg(not(windows))]
        {
            project.pyfaro(["type", &relative]).run().success();
        }
        project.pyfaro(["check"]).run().success();
        // ty ignores the exclusion for a named file on Windows (#109).
        #[cfg(not(windows))]
        {
            project.pyfaro(["check", &relative]).run().success();
        }

        // The same content in a file that is not excluded fails.
        project.write(
            format!("src/{}/included.py", project.package),
            "import os\nVALUE: int = \"x\"\nOTHER=2\n",
        );
        project.pyfaro(["lint"]).run().code(1);
        project.pyfaro(["check"]).run().code(1);
    });
}

#[test]
fn failing_quality_commands_write_nothing() {
    e2e_guard!();
    for_each_kind(|project| {
        let src = format!("src/{}", project.package);
        project.write(format!("{src}/lint_bad.py"), UNUSED_IMPORT);
        project.write(format!("{src}/fmt_bad.py"), UNFORMATTED);
        project.write(format!("{src}/type_bad.py"), TYPE_ERROR);
        project.write("tests/test_failure.py", FAILING_TEST);
        let before = project.snapshot();
        assert!(before.contains_key("uv.lock"));
        for args in [
            vec!["check"],
            vec!["lint"],
            vec!["type"],
            vec!["test"],
            vec!["fmt", "--check"],
        ] {
            project.pyfaro(&args).run().code(1);
            assert_eq!(project.snapshot(), before, "pyfaro {args:?} wrote");
        }
    });
}

// --------------------------------------------------------------------- lint

#[test]
fn lint_checks_explicit_files_and_directories_relative_to_the_cwd() {
    e2e_guard!();
    let sandbox = Sandbox::new();
    let project = sandbox.synced_project(Kind::Lib);
    let src = format!("src/{}", project.package);
    project.write(format!("{src}/extra.py"), UNUSED_IMPORT);
    let nested = project.path("src");

    let file = project
        .pyfaro(["lint"])
        .cwd(&nested)
        .arg(format!("{}/extra.py", project.package))
        .run();
    file.code(1);
    assert_printed(&file, "F401");
    project.pyfaro(["lint", project.package]).cwd(&nested).run().code(1);
    project
        .pyfaro(["lint", &format!("{}/__init__.py", project.package)])
        .cwd(&nested)
        .run()
        .success();
    project.pyfaro(["lint", "../tests"]).cwd(&nested).run().success();
    // Without paths, Ruff checks the whole project even from a nested cwd.
    project.pyfaro(["lint"]).cwd(&nested).run().code(1);
}

#[test]
fn lint_reports_ruff_being_missing() {
    e2e_guard!();
    let sandbox = Sandbox::new();
    let project = sandbox.synced_project(Kind::Lib);
    uninstall(&project, "ruff");
    empty_path(project.pyfaro(["lint"]), &sandbox)
        .run()
        .code(1)
        .stderr_contains(&format!("error: {}", missing("ruff")))
        .stdout_empty();
}

#[test]
fn lint_with_project_selects_exactly_that_directory() {
    e2e_guard!();
    let sandbox = Sandbox::new();
    let project = sandbox.synced_project(Kind::Lib);
    let src = format!("src/{}", project.package);
    project.write(format!("{src}/extra.py"), UNUSED_IMPORT);

    let out = sandbox.pyfaro(["lint", "--project"]).arg(&project.root).run();
    out.code(1);
    assert_printed(&out, "F401");
    // Paths stay relative to the cwd, not to the project root.
    sandbox
        .pyfaro(["lint", "--project"])
        .arg(&project.root)
        .arg(format!("project/{src}/__init__.py"))
        .run()
        .success();
    sandbox
        .pyfaro(["lint", "--project"])
        .arg(&project.root)
        .arg(format!("project/{src}/extra.py"))
        .run()
        .code(1);
    // The cwd is not inside a project, and `--project` is required there.
    sandbox.pyfaro(["lint"]).run().code(1).stderr_contains("pyproject.toml");
}

// ---------------------------------------------------------------------- fmt

#[test]
fn fmt_rewrites_files() {
    e2e_guard!();
    let sandbox = Sandbox::new();
    let project = sandbox.synced_project(Kind::Lib);
    let extra = format!("src/{}/extra.py", project.package);
    project.write(&extra, UNFORMATTED);
    project.pyfaro(["fmt"]).run().success();
    assert_eq!(project.read(&extra), "VALUE = 1\n");
    project.pyfaro(["fmt", "--check"]).run().success();
}

#[test]
fn fmt_check_reports_a_diff_without_writing() {
    e2e_guard!();
    let sandbox = Sandbox::new();
    let project = sandbox.synced_project(Kind::Lib);
    let extra = format!("src/{}/extra.py", project.package);
    project.write(&extra, UNFORMATTED);
    let before = project.snapshot();
    let out = project.pyfaro(["fmt", "--check"]).run();
    out.code(1);
    assert_printed(&out, "extra.py");
    assert_eq!(project.snapshot(), before);
    assert_eq!(project.read(&extra), UNFORMATTED);
}

#[test]
fn fmt_formats_only_the_named_paths_relative_to_the_cwd() {
    e2e_guard!();
    let sandbox = Sandbox::new();
    let project = sandbox.synced_project(Kind::Lib);
    let src = format!("src/{}", project.package);
    project.write(format!("{src}/named.py"), UNFORMATTED);
    project.write(format!("{src}/other.py"), UNFORMATTED);
    project
        .pyfaro(["fmt", "named.py"])
        .cwd(project.module("."))
        .run()
        .success();
    assert_eq!(project.read(format!("{src}/named.py")), "VALUE = 1\n");
    assert_eq!(project.read(format!("{src}/other.py")), UNFORMATTED);
    project
        .pyfaro(["fmt", "--check", "other.py"])
        .cwd(project.module("."))
        .run()
        .code(1);
    // A directory path formats everything below it.
    project
        .pyfaro(["fmt", project.package])
        .cwd(project.path("src"))
        .run()
        .success();
    assert_eq!(project.read(format!("{src}/other.py")), "VALUE = 1\n");
}

#[test]
fn fmt_leaves_excluded_files_alone_even_when_named() {
    e2e_guard!();
    let sandbox = Sandbox::new();
    let project = sandbox.synced_project(Kind::Lib);
    let relative = with_excluded_file(&project);
    let before = project.snapshot();
    project.pyfaro(["fmt", &relative]).run().success();
    project.pyfaro(["fmt"]).run().success();
    assert_eq!(project.snapshot(), before);
}

#[test]
fn fmt_with_project_and_missing_ruff() {
    e2e_guard!();
    let sandbox = Sandbox::new();
    let project = sandbox.synced_project(Kind::Lib);
    let extra = format!("src/{}/extra.py", project.package);
    project.write(&extra, UNFORMATTED);
    sandbox.pyfaro(["fmt", "--project"]).arg(&project.root).run().success();
    assert_eq!(project.read(&extra), "VALUE = 1\n");

    uninstall(&project, "ruff");
    empty_path(project.pyfaro(["fmt"]), &sandbox)
        .run()
        .code(1)
        .stderr_contains(&format!("error: {}", missing("ruff")));
}

// --------------------------------------------------------------------- type

#[test]
fn type_forwards_ty_diagnostics_and_exit_code() {
    e2e_guard!();
    let sandbox = Sandbox::new();
    let project = sandbox.synced_project(Kind::Lib);
    project.pyfaro(["type"]).run().success();
    project.write(format!("src/{}/extra.py", project.package), TYPE_ERROR);
    let out = project.pyfaro(["type"]).run();
    out.code(1).stderr_contains("error: ty failed (exit");
    assert_printed(&out, "invalid-assignment");
    assert_printed(&out, "extra.py:1:");
}

#[test]
fn type_resolves_project_dependencies_from_the_environment() {
    e2e_guard!();
    let sandbox = Sandbox::new();
    let project = sandbox.synced_project(Kind::Lib);
    sandbox.write(
        "typed-dep/pyproject.toml",
        "[project]\nname = \"typed-dep\"\nversion = \"0.1.0\"\nrequires-python = \">=3.12\"\ndependencies = []\n\n[build-system]\nrequires = [\"uv_build>=0.9.26,<0.10.0\"]\nbuild-backend = \"uv_build\"\n",
    );
    sandbox.write(
        "typed-dep/src/typed_dep/__init__.py",
        "def double(value: int) -> int:\n    return value * 2\n",
    );
    sandbox.write("typed-dep/src/typed_dep/py.typed", "");
    project.uv(["add"]).arg(sandbox.join("typed-dep")).run().success();

    let extra = format!("src/{}/uses_dep.py", project.package);
    project.write(
        &extra,
        "from typed_dep import double\n\nVALUE: int = double(1)\n",
    );
    project.pyfaro(["type"]).run().success();

    // The dependency's own types are used: an `int` is not a `str`.
    project.write(
        &extra,
        "from typed_dep import double\n\nVALUE: str = double(1)\n",
    );
    let out = project.pyfaro(["type"]).run();
    out.code(1);
    assert_printed(&out, "invalid-assignment");
    assert_printed(&out, "`int`");
    assert_not_printed(&out, "unresolved-import");
}

#[test]
fn type_checks_explicit_paths_relative_to_the_cwd() {
    e2e_guard!();
    let sandbox = Sandbox::new();
    let project = sandbox.synced_project(Kind::Lib);
    let src = format!("src/{}", project.package);
    project.write(format!("{src}/bad.py"), TYPE_ERROR);
    let nested = project.path("src");
    project
        .pyfaro(["type"])
        .cwd(&nested)
        .arg(format!("{}/bad.py", project.package))
        .run()
        .code(1);
    project
        .pyfaro(["type"])
        .cwd(&nested)
        .arg(format!("{}/__init__.py", project.package))
        .run()
        .success();
    project.pyfaro(["type"]).cwd(&nested).arg("../tests").run().success();
}

#[test]
fn type_skips_excluded_files_even_when_named() {
    e2e_guard!();
    let sandbox = Sandbox::new();
    let project = sandbox.synced_project(Kind::Lib);
    let _relative = with_excluded_file(&project);
    project.pyfaro(["type"]).run().success();
    // ty ignores the exclusion for a named file on Windows (#109).
    #[cfg(not(windows))]
    {
        project.pyfaro(["type", &_relative]).run().success();
    }
}

#[test]
fn type_reports_ty_being_missing() {
    e2e_guard!();
    let sandbox = Sandbox::new();
    let project = sandbox.synced_project(Kind::Lib);
    uninstall(&project, "ty");
    empty_path(project.pyfaro(["type"]), &sandbox)
        .run()
        .code(1)
        .stderr_contains(&format!("error: {}", missing("ty")))
        .stdout_empty();
}

#[test]
fn type_with_project_runs_from_another_directory() {
    e2e_guard!();
    let sandbox = Sandbox::new();
    let project = sandbox.synced_project(Kind::Lib);
    sandbox.pyfaro(["type", "--project"]).arg(&project.root).run().success();
    project.write(format!("src/{}/extra.py", project.package), TYPE_ERROR);
    sandbox.pyfaro(["type", "--project"]).arg(&project.root).run().code(1);
}

#[test]
fn quality_commands_use_the_environment_named_by_uv_project_environment() {
    e2e_guard!();
    let sandbox = Sandbox::new();
    let project = sandbox.project(Kind::Lib);
    let environment = sandbox.join("custom env");
    project
        .pyfaro(["sync"])
        .env("UV_PROJECT_ENVIRONMENT", &environment)
        .run()
        .success();
    assert!(!project.path(".venv").exists());
    let extra = format!("src/{}/extra.py", project.package);
    project.write(&extra, TYPE_ERROR);

    let with_env = |args: &[&str]| {
        empty_path(project.pyfaro(args), &sandbox)
            .env("UV_PROJECT_ENVIRONMENT", &environment)
            .run()
    };
    with_env(&["type"]).code(1);
    with_env(&["lint"]).success();
    with_env(&["fmt", "--check"]).success();
    with_env(&["test"]).success();
    with_env(&["check"]).code(1);

    // Without the variable there is no environment, and so no tools.
    empty_path(project.pyfaro(["type"]), &sandbox)
        .run()
        .code(1)
        .stderr_contains(&format!("error: {}", missing("ty")));
}

// -------------------------------------------------------------------- check

#[test]
fn check_summarizes_a_passing_project() {
    e2e_guard!();
    for_each_kind(|project| {
        let out = project.pyfaro(["check"]).run();
        out.success().stderr_contains(&summary(&[
            "format: ok",
            "lint: ok",
            "type: ok",
            "check: ok",
        ]));
        assert!(!out.stdout.contains("pyfaro check summary"));
    });
}

#[test]
fn check_marks_each_single_failure_kind_in_the_summary() {
    e2e_guard!();
    let sandbox = Sandbox::new();
    let project = sandbox.synced_project(Kind::Lib);
    let extra = format!("src/{}/extra.py", project.package);
    for (content, lines) in [
        (
            UNFORMATTED,
            [
                "format: failed (exit 1)",
                "lint: ok",
                "type: ok",
                "check: failed (format)",
            ],
        ),
        (
            UNUSED_IMPORT,
            [
                "format: ok",
                "lint: failed (exit 1)",
                "type: ok",
                "check: failed (lint)",
            ],
        ),
        (
            TYPE_ERROR,
            [
                "format: ok",
                "lint: ok",
                "type: failed (exit 1)",
                "check: failed (type)",
            ],
        ),
    ] {
        project.write(&extra, content);
        project
            .pyfaro(["check"])
            .run()
            .code(1)
            .stderr_contains(&summary(&lines));
    }
}

#[test]
fn check_runs_every_step_when_several_fail() {
    e2e_guard!();
    let sandbox = Sandbox::new();
    let project = sandbox.synced_project(Kind::Lib);
    project.write(
        format!("src/{}/extra.py", project.package),
        "import os\nVALUE: int = \"x\"\nOTHER=2\n",
    );
    let out = project.pyfaro(["check"]).run();
    out.code(1).stderr_contains(&summary(&[
        "format: failed (exit 1)",
        "lint: failed (exit 1)",
        "type: failed (exit 1)",
        "check: failed (format, lint, type)",
    ]));
    assert_printed(&out, "F401");
    assert_printed(&out, "invalid-assignment");
    assert_printed(&out, "would be reformatted");
}

#[test]
fn check_marks_a_missing_tool_unavailable_and_runs_the_other_steps() {
    e2e_guard!();
    let sandbox = Sandbox::new();
    let project = sandbox.synced_project(Kind::Lib);
    project.write(format!("src/{}/extra.py", project.package), UNUSED_IMPORT);
    uninstall(&project, "ty");
    empty_path(project.pyfaro(["check"]), &sandbox)
        .run()
        .code(1)
        .stderr_contains(&summary(&[
            "format: ok",
            "lint: failed (exit 1)",
            &format!("type: unavailable ({})", unavailable("ty")),
            "check: failed (lint, type)",
        ]));

    uninstall(&project, "ruff");
    empty_path(project.pyfaro(["check"]), &sandbox)
        .run()
        .code(1)
        .stderr_contains(&summary(&[
            &format!("format: unavailable ({})", unavailable("ruff")),
            &format!("lint: unavailable ({})", unavailable("ruff")),
            &format!("type: unavailable ({})", unavailable("ty")),
            "check: failed (format, lint, type)",
        ]));
}

#[test]
fn check_still_runs_type_when_ruff_is_missing() {
    e2e_guard!();
    let sandbox = Sandbox::new();
    let project = sandbox.synced_project(Kind::Lib);
    project.write(format!("src/{}/extra.py", project.package), TYPE_ERROR);
    uninstall(&project, "ruff");
    let out = empty_path(project.pyfaro(["check"]), &sandbox).run();
    out.code(1).stderr_contains(&summary(&[
        &format!("format: unavailable ({})", unavailable("ruff")),
        &format!("lint: unavailable ({})", unavailable("ruff")),
        "type: failed (exit 1)",
        "check: failed (format, lint, type)",
    ]));
    assert_printed(&out, "invalid-assignment");
}

#[test]
fn check_without_fix_writes_nothing() {
    e2e_guard!();
    let sandbox = Sandbox::new();
    let project = sandbox.synced_project(Kind::Lib);
    let src = format!("src/{}", project.package);
    project.write(format!("{src}/fixable.py"), "import os\nVALUE=1\n");
    let before = project.snapshot();
    project.pyfaro(["check"]).run().code(1);
    assert_eq!(project.snapshot(), before);
}

#[test]
fn check_fix_applies_lint_and_format_fixes_then_reverifies() {
    e2e_guard!();
    for_each_kind(|project| {
        let src = format!("src/{}", project.package);
        project.write(format!("{src}/fixable.py"), "import os\nVALUE=1\n");
        project.write(format!("{src}/format_only.py"), "OTHER=2\n");
        let out = project.pyfaro(["check", "--fix"]).run();
        out.success()
            .stderr_contains(
                "fix: format and lint can be fixed automatically; type has no automatic fixes\n",
            )
            .stderr_contains("fix lint: ok\nfix format: ok\n")
            .stderr_contains("fix: rechecking without writing\n")
            .stderr_contains(&summary(&[
                "format: ok",
                "lint: ok",
                "type: ok",
                "check: ok",
            ]));
        assert_eq!(project.read(format!("{src}/fixable.py")), "VALUE = 1\n");
        assert_eq!(
            project.read(format!("{src}/format_only.py")),
            "OTHER = 2\n"
        );
        project.pyfaro(["check"]).run().success();
    });
}

#[test]
fn check_fix_reports_what_it_cannot_fix() {
    e2e_guard!();
    for_each_kind(|project| {
        let src = format!("src/{}", project.package);
        project.write(format!("{src}/type_bad.py"), TYPE_ERROR);
        project.write(format!("{src}/fmt_bad.py"), "OTHER=2\n");
        let out = project.pyfaro(["check", "--fix"]).run();
        out.code(1).stderr_contains(&summary(&[
            "format: ok",
            "lint: ok",
            "type: failed (exit 1)",
            "check: failed (type)",
        ]));
        assert_eq!(project.read(format!("{src}/fmt_bad.py")), "OTHER = 2\n");
        assert_eq!(project.read(format!("{src}/type_bad.py")), TYPE_ERROR);
    });
}

#[test]
fn check_forwards_paths_and_project() {
    e2e_guard!();
    let sandbox = Sandbox::new();
    let project = sandbox.synced_project(Kind::Lib);
    let src = format!("src/{}", project.package);
    project.write(format!("{src}/bad.py"), UNUSED_IMPORT);
    let nested = project.path("src");

    project.pyfaro(["check"]).run().code(1);
    project
        .pyfaro(["check"])
        .cwd(&nested)
        .arg(format!("{}/__init__.py", project.package))
        .run()
        .success();
    project
        .pyfaro(["check"])
        .cwd(&nested)
        .arg(format!("{}/bad.py", project.package))
        .run()
        .code(1)
        .stderr_contains(&summary(&[
            "format: ok",
            "lint: failed (exit 1)",
            "type: ok",
            "check: failed (lint)",
        ]));
    sandbox
        .pyfaro(["check", "--project"])
        .arg(&project.root)
        .arg(format!("project/{src}/__init__.py"))
        .run()
        .success();
    sandbox.pyfaro(["check", "--project"]).arg(&project.root).run().code(1);
}

// --------------------------------------------------------------------- test

#[test]
fn test_exit_codes_follow_pytest() {
    e2e_guard!();
    for_each_kind(|project| {
        project.pyfaro(["test"]).run().success().stdout_contains("1 passed");

        let none = project.pyfaro(["test", "-k", "no_such_test"]).run();
        none.code(5)
            .stderr_contains("error: pytest failed (exit")
            .stdout_contains("1 deselected");

        let usage = project.pyfaro(["test", "--no-such-option"]).run();
        usage.code(4);
        assert_printed(&usage, "unrecognized arguments: --no-such-option");

        project.write("tests/test_failure.py", FAILING_TEST);
        project
            .pyfaro(["test"])
            .run()
            .code(1)
            .stdout_contains("1 failed, 1 passed");
    });
}

#[test]
fn test_selects_by_keyword_marker_and_node_id() {
    e2e_guard!();
    let sandbox = Sandbox::new();
    let project = sandbox.synced_project(Kind::Lib);
    let text = project.read("pyproject.toml").replace(
        "testpaths = [\"tests\"]\n",
        "testpaths = [\"tests\"]\nmarkers = [\"slow: slow tests\"]\naddopts = \"--strict-markers\"\n",
    );
    project.write("pyproject.toml", &text);
    project.write(
        "tests/test_select.py",
        "import pytest\n\n\ndef test_fast() -> None:\n    assert True\n\n\n@pytest.mark.slow\ndef test_slow() -> None:\n    assert False\n",
    );

    project
        .pyfaro(["test", "-k", "fast"])
        .run()
        .success()
        .stdout_contains("1 passed, 2 deselected");
    project
        .pyfaro(["test", "-k", "slow"])
        .run()
        .code(1)
        .stdout_contains("1 failed, 2 deselected");
    project
        .pyfaro(["test", "-m", "slow"])
        .run()
        .code(1)
        .stdout_contains("1 failed, 2 deselected");
    project
        .pyfaro(["test", "-m", "not slow"])
        .run()
        .success()
        .stdout_contains("2 passed, 1 deselected");
    project
        .pyfaro(["test", "tests/test_select.py::test_fast"])
        .run()
        .success()
        .stdout_contains("1 passed");
    project
        .pyfaro(["test", "tests/test_select.py::test_slow"])
        .run()
        .code(1)
        .stdout_contains("1 failed");
    project
        .pyfaro(["test", "tests/test_select.py::test_missing"])
        .run()
        .code(4);
}

#[test]
fn test_runs_with_the_project_root_as_the_working_directory() {
    e2e_guard!();
    let sandbox = Sandbox::new();
    let project = sandbox.synced_project(Kind::Lib);
    project.write(
        "tests/test_cwd.py",
        "import os\n\n\ndef test_cwd_is_the_project_root() -> None:\n    root = os.path.join(os.path.dirname(__file__), os.pardir)\n    assert os.path.samefile(os.getcwd(), root)\n",
    );
    // Forwarded paths are relative to the project root, not to the cwd.
    project
        .pyfaro(["test", "tests/test_cwd.py"])
        .cwd(project.module("."))
        .run()
        .success()
        .stdout_contains("1 passed");
    project
        .pyfaro(["test"])
        .cwd(project.path("tests"))
        .run()
        .success()
        .stdout_contains("2 passed");
}

#[test]
fn test_without_an_environment_asks_for_a_sync() {
    e2e_guard!();
    let sandbox = Sandbox::new();
    let project = sandbox.project(Kind::Lib);
    let before = project.snapshot();
    project
        .pyfaro(["test"])
        .run()
        .code(1)
        .stderr_contains(
            "error: pytest: project environment interpreter not found at ",
        )
        .stderr_contains("; run `pyfaro sync` first")
        .stdout_empty();
    assert!(!project.path(".venv").exists());
    assert_eq!(project.snapshot(), before);
}

#[test]
fn test_without_pytest_asks_to_add_it() {
    e2e_guard!();
    let sandbox = Sandbox::new();
    let project = sandbox.synced_project(Kind::Lib);
    uninstall(&project, "pytest");
    project
        .pyfaro(["test"])
        .run()
        .code(1)
        .stderr_contains(
            "error: pytest: not installed in the project environment (",
        )
        .stderr_contains("); run `pyfaro add --dev pytest` then `pyfaro sync`")
        .stdout_empty();
}

#[test]
fn test_with_project_and_the_separator() {
    e2e_guard!();
    let sandbox = Sandbox::new();
    let project = sandbox.synced_project(Kind::Lib);
    sandbox
        .pyfaro(["test", "--project"])
        .arg(&project.root)
        .run()
        .success()
        .stdout_contains("1 passed");
    sandbox
        .pyfaro(["test", "--project"])
        .arg(&project.root)
        .args(["--", "-k", "no_such_test"])
        .run()
        .code(5);
    // A leading `--` is removed, and everything after it reaches pytest.
    project.pyfaro(["test", "--", "-k", "no_such_test"]).run().code(5);
    project.pyfaro(["test", "--", "--no-such-option"]).run().code(4);
    let version = project.pyfaro(["test", "--", "--version"]).run();
    version.success();
    assert_printed(&version, "pytest 9.1.1");
    // Pyfaro's own options must come first; afterwards they are pytest's.
    project
        .pyfaro(["test", "-k", "hello", "--project"])
        .arg(&project.root)
        .run()
        .code(4);
}
