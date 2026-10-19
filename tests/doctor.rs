mod support;

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use tempfile::tempdir;

fn install(directory: &Path, name: &str) -> PathBuf {
    fs::create_dir_all(directory).unwrap();
    let binary =
        directory.join(format!("{name}{}", std::env::consts::EXE_SUFFIX));
    fs::copy(support::fixture_uv(), &binary).unwrap();
    binary
}

fn doctor_command(project: &Path, path: &Path, uv: &Path) -> Command {
    let mut command = isolated_command(project);
    command
        .arg("doctor")
        .arg("--project")
        .arg(project)
        .arg("--uv")
        .arg(uv)
        .env("PATH", path)
        .env_remove("VIRTUAL_ENV");
    for name in [
        "UV_NO_DEV",
        "UV_NO_DEFAULT_GROUPS",
        "UV_NO_GROUP",
        "UV_CONFIG_FILE",
        "UV_NO_CONFIG",
    ] {
        command.env_remove(name);
    }
    command
}

fn isolated_command(project: &Path) -> Command {
    let config_root = project.parent().unwrap_or(project);
    let user_config = config_root.join(".doctor-user-config");
    let system_config = config_root.join(".doctor-system-config");
    fs::create_dir_all(system_config.join("uv")).unwrap();
    let system_file = system_config.join("uv/uv.toml");
    if !system_file.exists() {
        fs::write(&system_file, "").unwrap();
    }
    let mut command = Command::new(env!("CARGO_BIN_EXE_pyfaro"));
    command
        .env("XDG_CONFIG_HOME", &user_config)
        .env("XDG_CONFIG_DIRS", &system_config)
        .env("APPDATA", &user_config)
        .env("PROGRAMDATA", &system_config);
    command
}

#[test]
fn user_uv_configuration_falls_back_and_reports_its_source() {
    let sandbox = tempdir().unwrap();
    let project = sandbox.path().join("project");
    let bin = sandbox.path().join("bin");
    fs::create_dir(&project).unwrap();
    let uv = install(&bin, "uv");
    install(&bin, "python");
    fs::write(project.join("pyproject.toml"), "[project]\nname = 'sample'\n[dependency-groups]\ndev = []\nquality = []\n").unwrap();
    let user_file = sandbox.path().join(".doctor-user-config/uv/uv.toml");
    fs::create_dir_all(user_file.parent().unwrap()).unwrap();
    fs::write(&user_file, "default-groups = ['quality']\n").unwrap();

    let output = doctor(&project, &bin, &uv);
    let report = text(&output);
    assert_eq!(output.status.code(), Some(0), "{output:?}");
    assert!(report.contains("group \"dev\" is not checked"), "{report}");
    assert!(!report.contains("group \"quality\" is not checked"), "{report}");

    fs::write(&user_file, "dev-dependencies = [42]\n").unwrap();
    let malformed = doctor(&project, &bin, &uv);
    let report = text(&malformed);
    assert_eq!(malformed.status.code(), Some(1), "{malformed:?}");
    assert!(
        report.contains(".doctor-user-config") && report.contains("uv.toml"),
        "{report}"
    );
    assert!(
        report
            .contains("dev-dependencies must contain only requirement strings"),
        "{report}"
    );
}

#[cfg(not(windows))]
#[test]
fn user_uv_configuration_defaults_to_home_config_directory() {
    let sandbox = tempdir().unwrap();
    let project = sandbox.path().join("project");
    let bin = sandbox.path().join("bin");
    fs::create_dir(&project).unwrap();
    let uv = install(&bin, "uv");
    install(&bin, "python");
    fs::write(project.join("pyproject.toml"), "[project]\nname = 'sample'\n[dependency-groups]\ndev = []\nquality = []\n").unwrap();
    let home = sandbox.path().join("home");
    let user_file = home.join(".config/uv/uv.toml");
    fs::create_dir_all(user_file.parent().unwrap()).unwrap();
    fs::write(&user_file, "default-groups = ['quality']\n").unwrap();

    let output = doctor_command(&project, &bin, &uv)
        .env_remove("XDG_CONFIG_HOME")
        .env("HOME", &home)
        .output()
        .unwrap();
    let report = text(&output);
    assert_eq!(output.status.code(), Some(0), "{output:?}");
    assert!(report.contains("group \"dev\" is not checked"), "{report}");
    assert!(!report.contains("group \"quality\" is not checked"), "{report}");
}

#[test]
fn uv_configuration_arrays_merge_across_project_user_and_system() {
    let sandbox = tempdir().unwrap();
    let project = sandbox.path().join("project");
    let bin = sandbox.path().join("bin");
    fs::create_dir(&project).unwrap();
    let uv = install(&bin, "uv");
    install(&bin, "python");
    fs::write(project.join("pyproject.toml"), "[project]\nname = 'sample'\n[dependency-groups]\nproject = []\nuser = []\nsystem = []\n").unwrap();
    fs::write(
        project.join("uv.toml"),
        "default-groups = ['project']\ndev-dependencies = []\n",
    )
    .unwrap();
    let user_file = sandbox.path().join(".doctor-user-config/uv/uv.toml");
    fs::create_dir_all(user_file.parent().unwrap()).unwrap();
    fs::write(&user_file, "default-groups = ['user']\ndev-dependencies = []\n")
        .unwrap();
    let system_file = sandbox.path().join(".doctor-system-config/uv/uv.toml");
    fs::create_dir_all(system_file.parent().unwrap()).unwrap();
    fs::write(
        &system_file,
        "default-groups = ['system']\ndev-dependencies = []\n",
    )
    .unwrap();

    let valid = doctor(&project, &bin, &uv);
    let report = text(&valid);
    assert_eq!(valid.status.code(), Some(0), "{valid:?}");
    for group in ["project", "user", "system"] {
        assert!(
            !report.contains(&format!("group \"{group}\" is not checked")),
            "{report}"
        );
    }

    fs::write(
        &system_file,
        "default-groups = ['system']\ndev-dependencies = [42]\n",
    )
    .unwrap();

    let output = doctor(&project, &bin, &uv);
    let report = text(&output);
    assert_eq!(output.status.code(), Some(1), "{output:?}");
    for group in ["project", "user", "system"] {
        assert!(
            !report.contains(&format!("group \"{group}\" is not checked")),
            "{report}"
        );
    }
    assert!(
        report.contains(".doctor-system-config") && report.contains("uv.toml"),
        "{report}"
    );
    assert!(
        report
            .contains("dev-dependencies must contain only requirement strings"),
        "{report}"
    );
}

#[test]
fn project_default_groups_override_lower_priority_all() {
    let sandbox = tempdir().unwrap();
    let project = sandbox.path().join("project");
    let bin = sandbox.path().join("bin");
    fs::create_dir(&project).unwrap();
    let uv = install(&bin, "uv");
    install(&bin, "python");
    fs::write(
        project.join("pyproject.toml"),
        "[project]\nname = 'sample'\n[dependency-groups]\nqa = []\ndev = []\n",
    )
    .unwrap();
    fs::write(project.join("uv.toml"), "default-groups = ['qa']\n").unwrap();
    let user_file = sandbox.path().join(".doctor-user-config/uv/uv.toml");
    fs::create_dir_all(user_file.parent().unwrap()).unwrap();
    fs::write(&user_file, "default-groups = 'all'\n").unwrap();

    let output = doctor(&project, &bin, &uv);
    let report = text(&output);
    assert_eq!(output.status.code(), Some(0), "{output:?}");
    assert!(report.contains("group \"dev\" is not checked"), "{report}");
    assert!(!report.contains("group \"qa\" is not checked"), "{report}");
}

#[test]
fn malformed_user_configuration_blocks_even_with_project_settings() {
    let sandbox = tempdir().unwrap();
    let project = sandbox.path().join("project");
    let bin = sandbox.path().join("bin");
    fs::create_dir(&project).unwrap();
    let uv = install(&bin, "uv");
    install(&bin, "python");
    fs::write(
        project.join("pyproject.toml"),
        "[project]\nname = 'sample'\n[dependency-groups]\ndev = []\n",
    )
    .unwrap();
    fs::write(project.join("uv.toml"), "default-groups = ['dev']\n").unwrap();
    let user_file = sandbox.path().join(".doctor-user-config/uv/uv.toml");
    fs::create_dir_all(user_file.parent().unwrap()).unwrap();
    fs::write(&user_file, "default-groups = [\n").unwrap();

    let malformed = doctor(&project, &bin, &uv);
    let report = text(&malformed);
    assert_eq!(malformed.status.code(), Some(1), "{malformed:?}");
    assert!(
        report.contains(".doctor-user-config") && report.contains("uv.toml"),
        "{report}"
    );
    assert!(!report.contains("OK: dependencies:"), "{report}");

    fs::remove_file(&user_file).unwrap();
    fs::create_dir(&user_file).unwrap();
    let unreadable = doctor(&project, &bin, &uv);
    let report = text(&unreadable);
    assert_eq!(unreadable.status.code(), Some(1), "{unreadable:?}");
    assert!(
        report.contains(".doctor-user-config") && report.contains("uv.toml"),
        "{report}"
    );
    assert!(report.contains("cannot read"), "{report}");
}

#[cfg(not(windows))]
#[test]
fn first_discovered_system_configuration_wins() {
    let sandbox = tempdir().unwrap();
    let project = sandbox.path().join("project");
    let bin = sandbox.path().join("bin");
    fs::create_dir(&project).unwrap();
    let uv = install(&bin, "uv");
    install(&bin, "python");
    fs::write(project.join("pyproject.toml"), "[project]\nname = 'sample'\n[dependency-groups]\ndev = []\nquality = []\n").unwrap();
    let first = sandbox.path().join("first/uv/uv.toml");
    let second = sandbox.path().join("second/uv/uv.toml");
    fs::create_dir_all(first.parent().unwrap()).unwrap();
    fs::create_dir_all(second.parent().unwrap()).unwrap();
    fs::write(&first, "default-groups = ['quality']\n").unwrap();
    fs::write(&second, "default-groups = ['dev']\n").unwrap();
    let paths = std::env::join_paths([
        first.parent().unwrap().parent().unwrap(),
        second.parent().unwrap().parent().unwrap(),
    ])
    .unwrap();

    let output = doctor_command(&project, &bin, &uv)
        .env("XDG_CONFIG_DIRS", paths)
        .output()
        .unwrap();
    let report = text(&output);
    assert_eq!(output.status.code(), Some(0), "{output:?}");
    assert!(report.contains("group \"dev\" is not checked"), "{report}");
    assert!(!report.contains("group \"quality\" is not checked"), "{report}");
}

fn doctor(project: &Path, path: &Path, uv: &Path) -> Output {
    doctor_command(project, path, uv).output().unwrap()
}

fn text(output: &Output) -> String {
    String::from_utf8(output.stdout.clone()).unwrap()
}

#[test]
fn healthy_fixture_reports_all_versions_without_writing_project() {
    let sandbox = tempdir().unwrap();
    let project = sandbox.path().join("project");
    let bin = sandbox.path().join("bin");
    fs::create_dir(&project).unwrap();
    let metadata = "[project]\nname = 'sample'\ndependencies = []\n";
    fs::write(project.join("pyproject.toml"), metadata).unwrap();
    let uv = install(&bin, "uv");
    for name in ["python", "ruff", "ty", "pytest"] {
        install(&bin, name);
    }
    let output = doctor(&project, &bin, &uv);
    assert_eq!(output.status.code(), Some(0), "{output:?}");
    let report = text(&output);
    for version in [
        "Python 3.12.1",
        "uv 0.9.26",
        "ruff 0.14.0",
        "ty 0.0.1-alpha.24",
        "pytest 8.4.2",
    ] {
        assert!(report.contains(version), "{report}");
    }
    assert!(report.contains("no blocking issues"), "{report}");
    assert!(output.stderr.is_empty(), "{output:?}");
    assert_eq!(
        fs::read_to_string(project.join("pyproject.toml")).unwrap(),
        metadata
    );
    assert!(!project.join(".venv").exists());
    assert!(!project.join("uv.lock").exists());
}

#[test]
fn missing_optional_tools_warn_but_missing_uv_or_python_block() {
    let sandbox = tempdir().unwrap();
    let project = sandbox.path().join("project");
    let bin = sandbox.path().join("bin");
    fs::create_dir_all(&project).unwrap();
    fs::write(project.join("pyproject.toml"), "[project]\nname = 'sample'\n")
        .unwrap();
    let python = install(&bin, "python");
    let uv = install(&bin, "uv");
    let optional = doctor(&project, &bin, &uv);
    assert_eq!(optional.status.code(), Some(0), "{optional:?}");
    let report = text(&optional);
    for name in ["ruff", "ty", "pytest"] {
        assert!(report.contains(&format!("WARN: {name}:")), "{report}");
    }
    fs::remove_file(&uv).unwrap();
    let no_uv = doctor(&project, &bin, &uv);
    assert_eq!(no_uv.status.code(), Some(1), "{no_uv:?}");
    assert!(text(&no_uv).contains("ERROR: uv:"));
    fs::remove_file(python).unwrap();
    let no_python = doctor(&project, &bin, &uv);
    assert_eq!(no_python.status.code(), Some(1), "{no_python:?}");
    assert!(text(&no_python).contains("ERROR: Python:"));
    assert!(
        String::from_utf8_lossy(&no_python.stderr).contains("blocking issue")
    );
}

#[test]
fn missing_project_is_a_blocking_finding() {
    let sandbox = tempdir().unwrap();
    let bin = sandbox.path().join("bin");
    let uv = install(&bin, "uv");
    install(&bin, "python");
    let output = isolated_command(&sandbox.path().join("missing-project"))
        .arg("doctor")
        .current_dir(sandbox.path())
        .arg("--uv")
        .arg(&uv)
        .env("PATH", &bin)
        .env_remove("VIRTUAL_ENV")
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(1), "{output:?}");
    assert!(text(&output).contains("ERROR: project: no pyproject.toml found"));
    assert!(text(&output).contains("Python 3.12.1"));
}

#[test]
fn invalid_configuration_conflicts_and_layout_are_actionable() {
    let sandbox = tempdir().unwrap();
    let project = sandbox.path().join("project");
    let bin = sandbox.path().join("bin");
    fs::create_dir_all(&project).unwrap();
    let uv = install(&bin, "uv");
    install(&bin, "python");
    let metadata = project.join("pyproject.toml");
    fs::write(
        &metadata,
        "[project]\nname = 'sample'\n[tool.pyfaro]\nschema-version = 2\n",
    )
    .unwrap();
    fs::write(project.join("src"), "wrong kind").unwrap();
    let invalid = doctor(&project, &bin, &uv);
    assert_eq!(invalid.status.code(), Some(1), "{invalid:?}");
    assert!(text(&invalid).contains("schema-version"));
    assert!(text(&invalid).contains("src must be a directory"), "{invalid:?}");

    fs::write(
        &metadata,
        "[project]\nname = 'sample'\n[tool.ruff]\nline-length = 88\n",
    )
    .unwrap();
    fs::write(project.join("ruff.toml"), "line-length = 90\n").unwrap();
    fs::write(project.join("src"), "wrong kind").unwrap();
    let problems = doctor(&project, &bin, &uv);
    assert_eq!(problems.status.code(), Some(1), "{problems:?}");
    let report = text(&problems);
    assert!(report.contains("[tool.ruff] and ruff.toml"), "{report}");
    assert!(report.contains("src must be a directory"), "{report}");
    assert!(report.contains("keep one source of configuration"), "{report}");

    fs::remove_file(project.join("ruff.toml")).unwrap();
    fs::remove_file(project.join("src")).unwrap();
    fs::write(
        &metadata,
        "[project]\nname = 'sample'\nrequires-python = '>=3.13'\n",
    )
    .unwrap();
    let incompatible = doctor(&project, &bin, &uv);
    assert_eq!(incompatible.status.code(), Some(1), "{incompatible:?}");
    assert!(text(&incompatible).contains("does not satisfy requires-python"));

    for dependencies in ["'invalid'", "[42]"] {
        fs::write(
            &metadata,
            format!(
                "[project]\nname = 'sample'\ndependencies = {dependencies}\n"
            ),
        )
        .unwrap();
        let invalid_dependencies = doctor(&project, &bin, &uv);
        assert_eq!(
            invalid_dependencies.status.code(),
            Some(1),
            "{invalid_dependencies:?}"
        );
        assert!(text(&invalid_dependencies).contains("ERROR: dependencies:"));
        assert!(!text(&invalid_dependencies).contains("OK: dependencies:"));
    }

    fs::write(
        &metadata,
        "[project]\nname = 'sample'\ndynamic = ['dependencies']\n",
    )
    .unwrap();
    let dynamic = doctor(&project, &bin, &uv);
    assert_eq!(dynamic.status.code(), Some(1), "{dynamic:?}");
    assert!(
        text(&dynamic)
            .contains("dynamic project dependencies cannot be checked")
    );
    assert!(!text(&dynamic).contains("OK: dependencies:"));

    fs::write(
        &metadata,
        "[project]\nname = 'sample'\ndependencies = []\ndynamic = ['dependencies']\n",
    )
    .unwrap();
    let conflicting_dynamic = doctor(&project, &bin, &uv);
    assert_eq!(
        conflicting_dynamic.status.code(),
        Some(1),
        "{conflicting_dynamic:?}"
    );
    assert!(
        text(&conflicting_dynamic)
            .contains("dynamic project dependencies cannot be checked")
    );
    assert!(!text(&conflicting_dynamic).contains("OK: dependencies:"));

    fs::write(
        &metadata,
        "[project]\nname = 'sample'\n[tool]\nuv = 'invalid'\n",
    )
    .unwrap();
    let malformed_uv = doctor(&project, &bin, &uv);
    assert_eq!(malformed_uv.status.code(), Some(1), "{malformed_uv:?}");
    assert!(
        text(&malformed_uv).contains("[tool.uv]")
            && text(&malformed_uv).contains("must be a table"),
        "{malformed_uv:?}"
    );
    assert!(!text(&malformed_uv).contains("OK: dependencies:"));
    fs::write(project.join("uv.toml"), "default-groups = []\n").unwrap();
    let malformed_ignored_uv = doctor(&project, &bin, &uv);
    assert_eq!(
        malformed_ignored_uv.status.code(),
        Some(0),
        "{malformed_ignored_uv:?}"
    );
    assert!(!text(&malformed_ignored_uv).contains("must be a table"));
    fs::remove_file(project.join("uv.toml")).unwrap();
    let no_config = doctor_command(&project, &bin, &uv)
        .env("UV_NO_CONFIG", "1")
        .output()
        .unwrap();
    assert_eq!(no_config.status.code(), Some(0), "{no_config:?}");
    assert!(!text(&no_config).contains("must be a table"));

    fs::write(&metadata, "[project]\nname = 'sample'\n").unwrap();
    fs::write(project.join(".python-version"), "3.11\n").unwrap();
    let pinned = doctor(&project, &bin, &uv);
    assert_eq!(pinned.status.code(), Some(1), "{pinned:?}");
    assert!(text(&pinned).contains("does not match .python-version"));
    fs::write(project.join(".python-version"), "3.12\n").unwrap();
    let matching = doctor(&project, &bin, &uv);
    assert_eq!(matching.status.code(), Some(0), "{matching:?}");
}

#[cfg(unix)]
#[test]
fn non_executable_project_python_is_blocking_without_dependencies() {
    use std::os::unix::fs::PermissionsExt;

    let sandbox = tempdir().unwrap();
    let project = sandbox.path().join("project");
    let bin = sandbox.path().join("bin");
    fs::create_dir_all(project.join(".venv/bin")).unwrap();
    fs::write(
        project.join("pyproject.toml"),
        "[project]\nname = 'sample'\ndependencies = []\n",
    )
    .unwrap();
    fs::write(project.join(".venv/bin/python"), "not executable").unwrap();
    fs::set_permissions(
        project.join(".venv/bin/python"),
        fs::Permissions::from_mode(0o644),
    )
    .unwrap();
    let uv = install(&bin, "uv");
    install(&bin, "python");

    let output = doctor(&project, &bin, &uv);
    let report = text(&output);
    assert_eq!(output.status.code(), Some(1), "{output:?}");
    assert!(report.contains(".venv/bin/python is not executable"), "{report}");
    assert!(report.contains("OK: Python:"), "{report}");
}

#[test]
fn group_names_are_normalized_for_defaults_and_includes() {
    let sandbox = tempdir().unwrap();
    let project = sandbox.path().join("project");
    let bin = sandbox.path().join("bin");
    fs::create_dir_all(&project).unwrap();
    let uv = install(&bin, "uv");
    install(&bin, "python");
    let metadata = project.join("pyproject.toml");
    fs::write(&metadata, "[project]\nname = 'sample'\n[dependency-groups]\nQA__Test = [{include-group = 'other..test'}]\nOther-Test = []\n[tool.uv]\ndefault-groups = ['qa.test']\n").unwrap();
    let output = doctor(&project, &bin, &uv);
    assert_eq!(output.status.code(), Some(0), "{output:?}");
    let report = text(&output);
    assert!(!report.contains("is not checked"), "{report}");
    assert!(report.contains("no blocking issues"), "{report}");
}

#[test]
fn project_uv_toml_takes_precedence_without_a_configuration_conflict() {
    let sandbox = tempdir().unwrap();
    let project = sandbox.path().join("project");
    let bin = sandbox.path().join("bin");
    fs::create_dir(&project).unwrap();
    let uv = install(&bin, "uv");
    install(&bin, "python");
    fs::write(project.join("pyproject.toml"), "[project]\nname = 'sample'\n[dependency-groups]\ndev = []\nquality = []\n[tool.uv]\ndefault-groups = ['dev']\n").unwrap();
    fs::write(project.join("uv.toml"), "default-groups = ['quality']\n")
        .unwrap();

    let output = doctor(&project, &bin, &uv);
    let report = text(&output);
    assert_eq!(output.status.code(), Some(0), "{output:?}");
    assert!(!report.contains("ERROR: configuration:"), "{report}");
    assert!(report.contains("group \"dev\" is not checked"), "{report}");
    assert!(!report.contains("group \"quality\" is not checked"), "{report}");
}

#[test]
fn uv_environment_configuration_controls_default_groups() {
    let sandbox = tempdir().unwrap();
    let project = sandbox.path().join("project");
    let bin = sandbox.path().join("bin");
    fs::create_dir(&project).unwrap();
    let uv = install(&bin, "uv");
    install(&bin, "python");
    fs::write(project.join("pyproject.toml"), "[project]\nname = 'sample'\n[dependency-groups]\ndev = []\nquality = []\n").unwrap();
    fs::write(project.join("uv.toml"), "default-groups = ['quality']\n")
        .unwrap();
    fs::write(project.join("override.toml"), "default-groups = ['dev']\n")
        .unwrap();
    let user_file = sandbox.path().join(".doctor-user-config/uv/uv.toml");
    fs::create_dir_all(user_file.parent().unwrap()).unwrap();
    fs::write(&user_file, "default-groups = [\n").unwrap();

    let override_output = doctor_command(&project, &bin, &uv)
        .current_dir(sandbox.path())
        .env("UV_CONFIG_FILE", "override.toml")
        .output()
        .unwrap();
    let report = text(&override_output);
    assert_eq!(override_output.status.code(), Some(0), "{override_output:?}");
    assert!(report.contains("group \"quality\" is not checked"), "{report}");
    assert!(!report.contains("group \"dev\" is not checked"), "{report}");

    let no_config = doctor_command(&project, &bin, &uv)
        .env("UV_NO_CONFIG", "1")
        .output()
        .unwrap();
    let report = text(&no_config);
    assert_eq!(no_config.status.code(), Some(0), "{no_config:?}");
    assert!(report.contains("group \"quality\" is not checked"), "{report}");
    assert!(!report.contains("group \"dev\" is not checked"), "{report}");

    let both = doctor_command(&project, &bin, &uv)
        .current_dir(sandbox.path())
        .env("UV_CONFIG_FILE", "override.toml")
        .env("UV_NO_CONFIG", "1")
        .output()
        .unwrap();
    assert_eq!(both.status.code(), Some(0), "{both:?}");
    assert!(
        text(&both).contains("group \"quality\" is not checked"),
        "{both:?}"
    );
}

#[test]
fn explicit_uv_configuration_errors_identify_the_file_and_action() {
    let sandbox = tempdir().unwrap();
    let project = sandbox.path().join("project");
    let bin = sandbox.path().join("bin");
    fs::create_dir(&project).unwrap();
    let uv = install(&bin, "uv");
    install(&bin, "python");
    fs::write(
        project.join("pyproject.toml"),
        "[project]\nname = 'sample'\ndependencies = []\n",
    )
    .unwrap();

    let missing = project.join("missing.toml");
    let missing_output = doctor_command(&project, &bin, &uv)
        .env("UV_CONFIG_FILE", &missing)
        .output()
        .unwrap();
    let report = text(&missing_output);
    assert_eq!(missing_output.status.code(), Some(1), "{missing_output:?}");
    assert!(report.contains(&missing.display().to_string()), "{report}");
    assert!(report.contains("UV_CONFIG_FILE"), "{report}");
    assert!(!report.contains("OK: dependencies:"), "{report}");

    let malformed = project.join("malformed.toml");
    fs::write(&malformed, "default-groups = [\n").unwrap();
    let malformed_output = doctor_command(&project, &bin, &uv)
        .env("UV_CONFIG_FILE", &malformed)
        .output()
        .unwrap();
    let report = text(&malformed_output);
    assert_eq!(malformed_output.status.code(), Some(1), "{malformed_output:?}");
    assert!(
        report.contains("invalid")
            && report.contains(&malformed.display().to_string()),
        "{report}"
    );
    assert!(report.contains("correct"), "{report}");

    let unreadable = project.join("directory.toml");
    fs::create_dir(&unreadable).unwrap();
    let unreadable_output = doctor_command(&project, &bin, &uv)
        .env("UV_CONFIG_FILE", &unreadable)
        .output()
        .unwrap();
    let report = text(&unreadable_output);
    assert_eq!(
        unreadable_output.status.code(),
        Some(1),
        "{unreadable_output:?}"
    );
    assert!(report.contains(&unreadable.display().to_string()), "{report}");
    assert!(report.contains("UV_CONFIG_FILE"), "{report}");
}

#[test]
fn pytest_conflicts_require_pytest_sections_in_shared_ini_files() {
    let sandbox = tempdir().unwrap();
    let project = sandbox.path().join("project");
    let bin = sandbox.path().join("bin");
    fs::create_dir(&project).unwrap();
    let uv = install(&bin, "uv");
    install(&bin, "python");
    fs::write(project.join("pyproject.toml"), "[project]\nname = 'sample'\n[tool.pytest.ini_options]\naddopts = '-q'\n").unwrap();
    fs::write(project.join("tox.ini"), "[tox]\nenv_list = py\n").unwrap();
    fs::write(project.join("setup.cfg"), "[metadata]\nname = sample\n")
        .unwrap();

    let unrelated = doctor(&project, &bin, &uv);
    assert_eq!(unrelated.status.code(), Some(0), "{unrelated:?}");
    assert!(!text(&unrelated).contains("ERROR: configuration:"));

    fs::write(project.join("setup.cfg"), "[pytest]\naddopts = -q\n").unwrap();
    let wrong_section = doctor(&project, &bin, &uv);
    assert_eq!(wrong_section.status.code(), Some(0), "{wrong_section:?}");

    fs::write(
        project.join("tox.ini"),
        "[tox]\nenv_list = py\n[pytest]\naddopts = -q\n",
    )
    .unwrap();
    let tox = doctor(&project, &bin, &uv);
    assert_eq!(tox.status.code(), Some(1), "{tox:?}");
    assert!(text(&tox).contains("[tool.pytest] and tox.ini"));

    fs::write(project.join("tox.ini"), "[pytest] # options\naddopts = -q\n")
        .unwrap();
    let commented_tox = doctor(&project, &bin, &uv);
    assert_eq!(commented_tox.status.code(), Some(1), "{commented_tox:?}");
    assert!(text(&commented_tox).contains("[tool.pytest] and tox.ini"));

    fs::remove_file(project.join("tox.ini")).unwrap();
    fs::write(
        project.join("setup.cfg"),
        "[metadata]\nname = sample\n[tool:pytest]\naddopts = -q\n",
    )
    .unwrap();
    let setup = doctor(&project, &bin, &uv);
    assert_eq!(setup.status.code(), Some(1), "{setup:?}");
    assert!(text(&setup).contains("[tool.pytest] and setup.cfg"));

    fs::remove_file(project.join("setup.cfg")).unwrap();
    fs::write(project.join(".pytest.ini"), "[pytest]\naddopts = -q\n").unwrap();
    let hidden = doctor(&project, &bin, &uv);
    assert_eq!(hidden.status.code(), Some(1), "{hidden:?}");
    assert!(text(&hidden).contains("[tool.pytest] and .pytest.ini"));

    fs::write(
        project.join("pyproject.toml"),
        "[project]\nname = 'sample'\n[tool.pytest.other]\nvalue = 1\n",
    )
    .unwrap();
    let unrelated = doctor(&project, &bin, &uv);
    assert_eq!(unrelated.status.code(), Some(0), "{unrelated:?}");
    assert!(!text(&unrelated).contains("ERROR: configuration:"));

    fs::write(
        project.join("pyproject.toml"),
        "[project]\nname = 'sample'\n[tool.pytest]\naddopts = ['-q']\n",
    )
    .unwrap();
    let native_toml = doctor(&project, &bin, &uv);
    assert_eq!(native_toml.status.code(), Some(1), "{native_toml:?}");
    assert!(text(&native_toml).contains("[tool.pytest] and .pytest.ini"));
    fs::remove_file(project.join(".pytest.ini")).unwrap();

    fs::write(project.join("pyproject.toml"), "[project]\nname = 'sample'\n[tool.pytest.ini_options]\naddopts = '-q'\n").unwrap();
    fs::write(project.join("pytest.toml"), "").unwrap();
    let pytest_toml = doctor(&project, &bin, &uv);
    assert_eq!(pytest_toml.status.code(), Some(1), "{pytest_toml:?}");
    assert!(text(&pytest_toml).contains("[tool.pytest] and pytest.toml"));
    fs::remove_file(project.join("pytest.toml")).unwrap();
    fs::write(project.join(".pytest.toml"), "").unwrap();
    let hidden_toml = doctor(&project, &bin, &uv);
    assert_eq!(hidden_toml.status.code(), Some(1), "{hidden_toml:?}");
    assert!(text(&hidden_toml).contains("[tool.pytest] and .pytest.toml"));
    fs::remove_file(project.join(".pytest.toml")).unwrap();

    fs::write(project.join("pyproject.toml"), "[project]\nname = 'sample'\n")
        .unwrap();
    fs::write(project.join("setup.cfg"), "[tool:pytest]\naddopts = -q\n")
        .unwrap();
    fs::write(project.join("tox.ini"), "[pytest]\naddopts = -q\n").unwrap();
    let shared = doctor(&project, &bin, &uv);
    assert_eq!(shared.status.code(), Some(1), "{shared:?}");
    assert!(
        text(&shared).contains("tox.ini and setup.cfg both configure pytest")
    );
}

#[test]
fn nearest_parent_uv_configuration_selects_default_groups() {
    let sandbox = tempdir().unwrap();
    let parent = sandbox.path().join("parent");
    let project = parent.join("project");
    let bin = sandbox.path().join("bin");
    fs::create_dir_all(&project).unwrap();
    let uv = install(&bin, "uv");
    install(&bin, "python");
    fs::write(project.join("pyproject.toml"), "[project]\nname = 'sample'\n[dependency-groups]\ndev = []\nquality = []\n").unwrap();
    fs::write(
        parent.join("pyproject.toml"),
        "[tool.uv]\ndefault-groups = ['dev']\n",
    )
    .unwrap();
    fs::write(parent.join("uv.toml"), "default-groups = ['quality']\n")
        .unwrap();

    let output = doctor(&project, &bin, &uv);
    let report = text(&output);
    assert_eq!(output.status.code(), Some(0), "{output:?}");
    assert!(report.contains("group \"dev\" is not checked"), "{report}");
    assert!(!report.contains("group \"quality\" is not checked"), "{report}");

    fs::remove_file(parent.join("uv.toml")).unwrap();
    fs::write(
        parent.join("pyproject.toml"),
        "[tool.uv]\ndefault-groups = ['quality']\n",
    )
    .unwrap();
    let inherited = doctor(&project, &bin, &uv);
    let report = text(&inherited);
    assert_eq!(inherited.status.code(), Some(0), "{inherited:?}");
    assert!(report.contains("group \"dev\" is not checked"), "{report}");
    assert!(!report.contains("group \"quality\" is not checked"), "{report}");

    fs::write(project.join("pyproject.toml"), "[project]\nname = 'sample'\n[dependency-groups]\ndev = []\nquality = []\n[tool.uv]\ndefault-groups = ['dev']\n").unwrap();
    let nearer = doctor(&project, &bin, &uv);
    let report = text(&nearer);
    assert_eq!(nearer.status.code(), Some(0), "{nearer:?}");
    assert!(report.contains("group \"quality\" is not checked"), "{report}");
    assert!(!report.contains("group \"dev\" is not checked"), "{report}");
}

#[test]
fn unselected_legacy_dev_dependencies_are_validated_at_their_source() {
    let sandbox = tempdir().unwrap();
    let project = sandbox.path().join("project");
    let bin = sandbox.path().join("bin");
    fs::create_dir(&project).unwrap();
    let uv = install(&bin, "uv");
    install(&bin, "python");
    fs::write(
        project.join("pyproject.toml"),
        "[project]\nname = 'sample'\n[dependency-groups]\nquality = []\n",
    )
    .unwrap();
    fs::write(
        project.join("uv.toml"),
        "default-groups = ['quality']\ndev-dependencies = [42]\n",
    )
    .unwrap();

    let output = doctor(&project, &bin, &uv);
    let report = text(&output);
    assert_eq!(output.status.code(), Some(1), "{output:?}");
    assert!(
        report.contains(
            "uv.toml dev-dependencies must contain only requirement strings"
        ),
        "{report}"
    );
    assert!(!report.contains("correct pyproject.toml"), "{report}");

    fs::write(project.join("uv.toml"), "default-groups = [\n").unwrap();
    let invalid_config = doctor(&project, &bin, &uv);
    let report = text(&invalid_config);
    assert_eq!(invalid_config.status.code(), Some(1), "{invalid_config:?}");
    assert!(
        report.contains("invalid") && report.contains("uv.toml"),
        "{report}"
    );
    assert!(!report.contains("OK: dependencies:"), "{report}");
}

#[test]
fn invalid_and_unselected_groups_are_blocking_without_distribution_checks() {
    let sandbox = tempdir().unwrap();
    let project = sandbox.path().join("project");
    let bin = sandbox.path().join("bin");
    fs::create_dir_all(&project).unwrap();
    let uv = install(&bin, "uv");
    install(&bin, "python");
    let metadata = project.join("pyproject.toml");
    let cases = [
        (
            "[tool.uv]\ndefault-groups = ['missing']\n",
            "group \"missing\" is missing",
        ),
        (
            "[dependency-groups]\nqa = []\n[tool.uv]\ndefault-groups = ['missing']\n",
            "group \"missing\" is missing",
        ),
        (
            "[dependency-groups]\ndev = []\nQA_Test = []\nqa-test = []\n",
            "duplicate dependency group names",
        ),
        (
            "[dependency-groups]\ndev = []\n'bad_' = []\n",
            "group name \"bad_\" is invalid",
        ),
        (
            "[dependency-groups]\ndev = []\nother = [42]\n",
            "group \"other\" contains an invalid entry",
        ),
        (
            "[dependency-groups]\ndev = []\nother = [{include-group = 'cycle'}]\ncycle = [{include-group = 'OTHER'}]\n",
            "dependency group cycle",
        ),
        (
            "[dependency-groups]\ndev = []\nother = [{include-group = 'missing'}]\n",
            "group \"missing\" is missing",
        ),
        (
            "[dependency-groups]\ndev = []\nother = [{include-group = 'dev', extra = true}]\n",
            "group \"other\" contains an invalid entry",
        ),
    ];
    for (groups, expected) in cases {
        fs::write(&metadata, format!("[project]\nname = 'sample'\n{groups}"))
            .unwrap();
        let output = doctor(&project, &bin, &uv);
        let report = text(&output);
        assert_eq!(output.status.code(), Some(1), "{output:?}");
        assert!(report.contains(expected), "{report}");
        assert!(!report.contains("OK: dependencies:"), "{report}");
    }
    fs::write(
        &metadata,
        "[project]\nname = 'sample'\n[tool.uv]\ndefault-groups = ['missing']\n",
    )
    .unwrap();
    let ignored_defaults = isolated_command(&project)
        .arg("doctor")
        .arg("--project")
        .arg(&project)
        .arg("--uv")
        .arg(&uv)
        .env("PATH", &bin)
        .env_remove("VIRTUAL_ENV")
        .env("UV_NO_DEFAULT_GROUPS", "1")
        .output()
        .unwrap();
    assert_eq!(ignored_defaults.status.code(), Some(1), "{ignored_defaults:?}");
    assert!(text(&ignored_defaults).contains("group \"missing\" is missing"));
}

#[test]
fn real_offline_uv_environment_checks_missing_runtime_distributions() {
    let sandbox = tempdir().unwrap();
    let project = sandbox.path().join("project");
    fs::create_dir(&project).unwrap();
    let metadata = project.join("pyproject.toml");
    fs::write(&metadata, "[project]\nname = 'sample'\nversion = '1.0.0'\nrequires-python = '>=3.10'\ndependencies = []\n").unwrap();
    let sync = isolated_command(&project)
        .arg("sync")
        .arg("--offline")
        .arg("--project")
        .arg(&project)
        .env("UV_CACHE_DIR", sandbox.path().join("cache"))
        .output()
        .unwrap();
    assert!(sync.status.success(), "{sync:?}");
    let uv = PathBuf::from("uv");
    let path = std::env::var_os("PATH").unwrap();
    let interpreter = project.join(".venv").join(if cfg!(windows) {
        "Scripts/python.exe"
    } else {
        "bin/python"
    });
    let version_output =
        Command::new(&interpreter).arg("--version").output().unwrap();
    assert!(version_output.status.success(), "{version_output:?}");
    let version = String::from_utf8(version_output.stdout).unwrap();
    let major_minor = version
        .trim()
        .strip_prefix("Python ")
        .unwrap()
        .split('.')
        .take(2)
        .collect::<Vec<_>>()
        .join(".");
    let healthy = isolated_command(&project)
        .arg("doctor")
        .arg("--project")
        .arg(&project)
        .env("PATH", &path)
        .env_remove("VIRTUAL_ENV")
        .output()
        .unwrap();
    assert_eq!(healthy.status.code(), Some(0), "{healthy:?}");
    fs::write(&metadata, "[project]\nname = 'sample'\nversion = '1.0.0'\nrequires-python = '>=3.10'\ndependencies = ['missing-doctor-fixture-package==1.0']\n").unwrap();
    let before = fs::read(project.join("uv.lock")).unwrap();
    let missing = isolated_command(&project)
        .arg("doctor")
        .arg("--project")
        .arg(&project)
        .arg("--uv")
        .arg(&uv)
        .env("PATH", &path)
        .env_remove("VIRTUAL_ENV")
        .output()
        .unwrap();
    assert_eq!(missing.status.code(), Some(1), "{missing:?}");
    assert!(
        text(&missing)
            .contains("missing-doctor-fixture-package is missing from .venv")
    );
    assert_eq!(fs::read(project.join("uv.lock")).unwrap(), before);
    assert!(!project.join("pyfaro.lock").exists());

    fs::write(&metadata, format!("[project]\nname = 'sample'\nversion = '1.0.0'\nrequires-python = '>=3.10'\ndependencies = ['missing-doctor-fixture-package==1.0; python_version == \"{major_minor}\"', 'ignored-doctor-fixture-package; python_version < \"0\"']\n")).unwrap();
    let marked = isolated_command(&project)
        .arg("doctor")
        .arg("--project")
        .arg(&project)
        .env("PATH", &path)
        .env_remove("VIRTUAL_ENV")
        .output()
        .unwrap();
    assert_eq!(marked.status.code(), Some(1), "{marked:?}");
    let report = text(&marked);
    assert!(
        report.contains("missing-doctor-fixture-package is missing from .venv"),
        "{report}"
    );
    assert!(
        !report.contains("ignored-doctor-fixture-package is missing"),
        "{report}"
    );
    assert_eq!(fs::read(project.join("uv.lock")).unwrap(), before);

    fs::write(&metadata, "[project]\nname = 'sample'\nversion = '1.0.0'\ndependencies = ['missing-doctor-valid-package==1.0', 'broken =>1']\n").unwrap();
    let malformed = isolated_command(&project)
        .arg("doctor")
        .arg("--project")
        .arg(&project)
        .env("PATH", &path)
        .env_remove("VIRTUAL_ENV")
        .output()
        .unwrap();
    assert_eq!(malformed.status.code(), Some(1), "{malformed:?}");
    assert!(text(&malformed).contains("invalid requirement"), "{malformed:?}");
    assert!(!text(&malformed).contains("OK: dependencies:"));

    fs::write(&metadata, "[project]\nname = 'sample'\nversion = '1.0.0'\n[dependency-groups]\ndev = []\nother = ['broken =>1']\n").unwrap();
    let malformed_unselected = isolated_command(&project)
        .arg("doctor")
        .arg("--project")
        .arg(&project)
        .env("PATH", &path)
        .env_remove("VIRTUAL_ENV")
        .output()
        .unwrap();
    assert_eq!(
        malformed_unselected.status.code(),
        Some(1),
        "{malformed_unselected:?}"
    );
    assert!(text(&malformed_unselected).contains("invalid requirement"));
    assert!(!text(&malformed_unselected).contains("OK: dependencies:"));

    fs::write(&metadata, "[project]\nname = 'sample'\nversion = '1.0.0'\n[dependency-groups]\ndev = ['missing-doctor-dev-package==1.0']\nquality = ['missing-doctor-quality-package==1.0']\n").unwrap();
    let grouped = isolated_command(&project)
        .arg("doctor")
        .arg("--project")
        .arg(&project)
        .env("PATH", &path)
        .env_remove("VIRTUAL_ENV")
        .output()
        .unwrap();
    assert_eq!(grouped.status.code(), Some(1), "{grouped:?}");
    let report = text(&grouped);
    assert!(
        report.contains("missing-doctor-dev-package is missing from .venv"),
        "{report}"
    );
    assert!(report.contains("group \"quality\" is not checked"), "{report}");
    assert!(
        !report.contains("missing-doctor-quality-package is missing"),
        "{report}"
    );

    fs::write(&metadata, "[project]\nname = 'sample'\nversion = '1.0.0'\n[dependency-groups]\ndev = ['missing-doctor-dev-package==1.0']\nquality = ['missing-doctor-quality-package==1.0']\n[tool.uv]\ndefault-groups = ['quality']\n").unwrap();
    let selected = isolated_command(&project)
        .arg("doctor")
        .arg("--project")
        .arg(&project)
        .env("PATH", &path)
        .env_remove("VIRTUAL_ENV")
        .output()
        .unwrap();
    assert_eq!(selected.status.code(), Some(1), "{selected:?}");
    let report = text(&selected);
    assert!(
        report.contains("missing-doctor-quality-package is missing from .venv"),
        "{report}"
    );
    assert!(
        !report.contains("missing-doctor-dev-package is missing"),
        "{report}"
    );

    fs::write(&metadata, "[project]\nname = 'sample'\nversion = '1.0.0'\n[dependency-groups]\ndev = ['missing-doctor-dev-package==1.0']\nquality = ['missing-doctor-quality-package==1.0']\n").unwrap();
    let with_uv_env = |name: &str, value: &str| {
        isolated_command(&project)
            .arg("doctor")
            .arg("--project")
            .arg(&project)
            .env("PATH", &path)
            .env_remove("VIRTUAL_ENV")
            .env("UV_NO_DEV", "false")
            .env("UV_NO_DEFAULT_GROUPS", "false")
            .env_remove("UV_NO_GROUP")
            .env(name, value)
            .output()
            .unwrap()
    };
    let no_dev = with_uv_env("UV_NO_DEV", "1");
    let report = text(&no_dev);
    assert_eq!(no_dev.status.code(), Some(0), "{no_dev:?}");
    assert!(
        !report.contains("missing-doctor-dev-package is missing"),
        "{report}"
    );
    let short_true = with_uv_env("UV_NO_DEV", "y");
    let report = text(&short_true);
    assert_eq!(short_true.status.code(), Some(0), "{short_true:?}");
    assert!(
        !report.contains("missing-doctor-dev-package is missing"),
        "{report}"
    );
    let no_defaults = with_uv_env("UV_NO_DEFAULT_GROUPS", "1");
    assert_eq!(no_defaults.status.code(), Some(0), "{no_defaults:?}");
    let no_group = with_uv_env("UV_NO_GROUP", "dev");
    assert_eq!(no_group.status.code(), Some(0), "{no_group:?}");

    fs::write(&metadata, "[project]\nname = 'sample'\nversion = '1.0.0'\n[dependency-groups]\ndev = ['missing-doctor-dev-package==1.0']\nquality = ['missing-doctor-quality-package==1.0']\n[tool.uv]\ndefault-groups = ['dev', 'quality']\n").unwrap();
    let no_dev = with_uv_env("UV_NO_DEV", "1");
    let report = text(&no_dev);
    assert_eq!(no_dev.status.code(), Some(1), "{no_dev:?}");
    assert!(
        report.contains("missing-doctor-quality-package is missing"),
        "{report}"
    );
    assert!(
        !report.contains("missing-doctor-dev-package is missing"),
        "{report}"
    );
    let no_group = with_uv_env("UV_NO_GROUP", "DEV quality");
    let report = text(&no_group);
    assert_eq!(no_group.status.code(), Some(0), "{no_group:?}");
    assert!(!report.contains("is missing from .venv"), "{report}");
    fs::write(&metadata, "[project]\nname = 'sample'\nversion = '1.0.0'\n[dependency-groups]\ndev = []\nquality = ['broken =>1']\n[tool.uv]\ndefault-groups = ['dev', 'quality']\n").unwrap();
    let invalid_excluded = with_uv_env("UV_NO_DEFAULT_GROUPS", "1");
    assert_eq!(invalid_excluded.status.code(), Some(1), "{invalid_excluded:?}");
    assert!(text(&invalid_excluded).contains("invalid requirement"));

    fs::write(&metadata, "[project]\nname = 'sample'\nversion = '1.0.0'\n[dependency-groups]\ndev = [{include-group = 'quality'}]\nquality = ['missing-doctor-quality-package==1.0']\n").unwrap();
    let included = isolated_command(&project)
        .arg("doctor")
        .arg("--project")
        .arg(&project)
        .env("PATH", &path)
        .env_remove("VIRTUAL_ENV")
        .output()
        .unwrap();
    assert_eq!(included.status.code(), Some(1), "{included:?}");
    let report = text(&included);
    assert!(
        report.contains("missing-doctor-quality-package is missing from .venv"),
        "{report}"
    );
    assert!(!report.contains("group \"quality\" is not checked"), "{report}");
    let excluded_include = with_uv_env("UV_NO_GROUP", "quality");
    let report = text(&excluded_include);
    assert_eq!(excluded_include.status.code(), Some(0), "{excluded_include:?}");
    assert!(
        !report.contains("missing-doctor-quality-package is missing"),
        "{report}"
    );

    fs::write(&metadata, "[project]\nname = 'sample'\nversion = '1.0.0'\n[dependency-groups]\na = [{include-group = 'b'}]\nb = [{include-group = 'c'}]\nc = ['missing-doctor-selected-package==1.0']\n[tool.uv]\ndefault-groups = ['a', 'c']\n").unwrap();
    let separately_selected = doctor_command(&project, Path::new(&path), &uv)
        .env("UV_NO_GROUP", "b")
        .output()
        .unwrap();
    let report = text(&separately_selected);
    assert_eq!(separately_selected.status.code(), Some(1), "{report}");
    assert!(
        report
            .contains("missing-doctor-selected-package is missing from .venv"),
        "{report}"
    );
    assert!(!report.contains("group \"c\" is not checked"), "{report}");

    fs::write(&metadata, "[project]\nname = 'sample'\nversion = '1.0.0'\n[dependency-groups]\nQA__Test = [{include-group = 'other..test'}]\nOther-Test = ['missing-doctor-normalized-package==1.0']\n[tool.uv]\ndefault-groups = ['qa.test']\n").unwrap();
    let normalized = isolated_command(&project)
        .arg("doctor")
        .arg("--project")
        .arg(&project)
        .env("PATH", &path)
        .env_remove("VIRTUAL_ENV")
        .output()
        .unwrap();
    assert_eq!(normalized.status.code(), Some(1), "{normalized:?}");
    let report = text(&normalized);
    assert!(
        report.contains(
            "missing-doctor-normalized-package is missing from .venv"
        ),
        "{report}"
    );
    assert!(!report.contains("is not checked"), "{report}");

    fs::write(&metadata, "dependency-groups = { dev = ['missing-doctor-inline-package==1.0'] }\n[project]\nname = 'sample'\nversion = '1.0.0'\n").unwrap();
    let inline = isolated_command(&project)
        .arg("doctor")
        .arg("--project")
        .arg(&project)
        .env("PATH", &path)
        .env_remove("VIRTUAL_ENV")
        .output()
        .unwrap();
    assert_eq!(inline.status.code(), Some(1), "{inline:?}");
    assert!(
        text(&inline)
            .contains("missing-doctor-inline-package is missing from .venv"),
        "{inline:?}"
    );

    fs::write(&metadata, "[project]\nname = 'sample'\nversion = '1.0.0'\n[dependency-groups]\ndev = ['missing-doctor-dev-package==1.0']\nquality = ['missing-doctor-uvtoml-package==1.0']\n").unwrap();
    fs::write(project.join("uv.toml"), "default-groups = ['quality']\n")
        .unwrap();
    let configured = isolated_command(&project)
        .arg("doctor")
        .arg("--project")
        .arg(&project)
        .env("PATH", &path)
        .env_remove("VIRTUAL_ENV")
        .output()
        .unwrap();
    assert_eq!(configured.status.code(), Some(1), "{configured:?}");
    let report = text(&configured);
    assert!(
        report.contains("missing-doctor-uvtoml-package is missing from .venv"),
        "{report}"
    );
    assert!(
        !report.contains("missing-doctor-dev-package is missing"),
        "{report}"
    );

    fs::write(&metadata, "[project]\nname = 'sample'\nversion = '1.0.0'\n[dependency-groups]\ndev = []\nquality = []\n").unwrap();
    fs::write(
        project.join("uv.toml"),
        "default-groups = ['quality']\ndev-dependencies = ['broken =>1']\n",
    )
    .unwrap();
    let malformed_legacy = isolated_command(&project)
        .arg("doctor")
        .arg("--project")
        .arg(&project)
        .env("PATH", &path)
        .env_remove("VIRTUAL_ENV")
        .output()
        .unwrap();
    assert_eq!(malformed_legacy.status.code(), Some(1), "{malformed_legacy:?}");
    let report = text(&malformed_legacy);
    assert!(report.contains("invalid requirement \"broken =>1\""), "{report}");
    assert!(
        report.contains(&format!(
            "correct {}",
            fs::canonicalize(project.join("uv.toml")).unwrap().display()
        )),
        "{report}"
    );
    assert!(!report.contains("correct pyproject.toml"), "{report}");

    fs::write(project.join("uv.toml"), "default-groups = ['quality']\ndev-dependencies = ['missing-doctor-legacy-package==1.0']\n").unwrap();
    let unselected_legacy = isolated_command(&project)
        .arg("doctor")
        .arg("--project")
        .arg(&project)
        .env("PATH", &path)
        .env_remove("VIRTUAL_ENV")
        .output()
        .unwrap();
    assert_eq!(
        unselected_legacy.status.code(),
        Some(0),
        "{unselected_legacy:?}"
    );
    assert!(
        !text(&unselected_legacy)
            .contains("missing-doctor-legacy-package is missing")
    );

    fs::write(project.join("uv.toml"), "default-groups = ['dev']\ndev-dependencies = ['missing-doctor-legacy-package==1.0']\n").unwrap();
    let selected_legacy = isolated_command(&project)
        .arg("doctor")
        .arg("--project")
        .arg(&project)
        .env("PATH", &path)
        .env_remove("VIRTUAL_ENV")
        .output()
        .unwrap();
    assert_eq!(selected_legacy.status.code(), Some(1), "{selected_legacy:?}");
    assert!(
        text(&selected_legacy)
            .contains("missing-doctor-legacy-package is missing from .venv")
    );

    fs::write(&metadata, "[project]\nname = 'sample'\nversion = '1.0.0'\n[dependency-groups]\nqa = [{include-group = 'dev'}]\ndev = []\n").unwrap();
    fs::write(project.join("uv.toml"), "default-groups = ['qa']\ndev-dependencies = ['missing-doctor-legacy-package==1.0']\n").unwrap();
    let included_legacy =
        doctor_command(&project, Path::new(&path), &uv).output().unwrap();
    assert_eq!(included_legacy.status.code(), Some(1), "{included_legacy:?}");
    assert!(
        text(&included_legacy)
            .contains("missing-doctor-legacy-package is missing from .venv")
    );

    fs::write(project.join("uv.toml"), "default-groups = ['dev']\ndev-dependencies = ['missing-doctor-legacy-package==1.0']\n").unwrap();
    fs::write(&metadata, "[project]\nname = 'sample'\nversion = '1.0.0'\n")
        .unwrap();
    let legacy_only = isolated_command(&project)
        .arg("doctor")
        .arg("--project")
        .arg(&project)
        .env("PATH", &path)
        .env_remove("VIRTUAL_ENV")
        .output()
        .unwrap();
    let report = text(&legacy_only);
    assert_eq!(legacy_only.status.code(), Some(1), "{legacy_only:?}");
    assert!(
        report.contains("missing-doctor-legacy-package is missing from .venv"),
        "{report}"
    );
    assert!(!report.contains("group \"dev\" is missing"), "{report}");
}
