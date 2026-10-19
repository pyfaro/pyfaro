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
        .env("PROGRAMDATA", &system_config)
        .env_remove("UV_PROJECT_ENVIRONMENT");
    command
}

#[test]
fn workspace_member_reports_unsupported_layout() {
    let sandbox = tempdir().unwrap();
    let workspace = sandbox.path().join("workspace");
    let member = workspace.join("member");
    let bin = sandbox.path().join("bin");
    fs::create_dir_all(&member).unwrap();
    let uv = install(&bin, "uv");
    install(&bin, "python");
    fs::write(
        workspace.join("pyproject.toml"),
        "[project]\nname = 'workspace'\nversion = '0.1.0'\n[tool.uv.workspace]\nmembers = ['member']\n",
    )
    .unwrap();
    fs::write(
        member.join("pyproject.toml"),
        "[project]\nname = 'member'\nversion = '0.1.0'\n[dependency-groups]\ndev = []\n",
    )
    .unwrap();

    let output = doctor(&member, &bin, &uv);
    let report = text(&output);
    assert_eq!(output.status.code(), Some(1), "{output:?}");
    assert!(
        report.contains("uv workspace") && report.contains("not supported"),
        "{report}"
    );
    assert!(!report.contains("OK: dependencies:"), "{report}");

    fs::write(member.join("uv.toml"), "offline = true\n").unwrap();
    let with_child_config = doctor(&member, &bin, &uv);
    assert_eq!(
        with_child_config.status.code(),
        Some(1),
        "{with_child_config:?}"
    );
    assert!(
        text(&with_child_config).contains("uv workspace"),
        "{with_child_config:?}"
    );
}

#[test]
fn workspace_root_reports_unsupported_layout() {
    let sandbox = tempdir().unwrap();
    let project = sandbox.path().join("workspace");
    let bin = sandbox.path().join("bin");
    fs::create_dir(&project).unwrap();
    let uv = install(&bin, "uv");
    install(&bin, "python");
    fs::write(
        project.join("pyproject.toml"),
        "[project]\nname = 'workspace'\nversion = '0.1.0'\n[tool.uv.workspace]\nmembers = []\n",
    )
    .unwrap();

    let output = doctor(&project, &bin, &uv);
    assert_eq!(output.status.code(), Some(1), "{output:?}");
    assert!(text(&output).contains("uv workspace"));
}

#[test]
fn invalid_uv_workspace_and_managed_types_are_blocking() {
    let sandbox = tempdir().unwrap();
    let workspace = sandbox.path().join("workspace");
    let project = workspace.join("member");
    let bin = sandbox.path().join("bin");
    fs::create_dir_all(&project).unwrap();
    let uv = install(&bin, "uv");
    install(&bin, "python");
    fs::write(
        project.join("pyproject.toml"),
        "[project]\nname = 'member'\nversion = '0.1.0'\n",
    )
    .unwrap();

    fs::write(
        workspace.join("pyproject.toml"),
        "[tool.uv]\nworkspace = true\n",
    )
    .unwrap();
    let invalid_workspace = doctor(&project, &bin, &uv);
    assert_eq!(
        invalid_workspace.status.code(),
        Some(1),
        "{invalid_workspace:?}"
    );
    assert!(
        text(&invalid_workspace).contains("workspace")
            && text(&invalid_workspace).contains("must be a table")
    );

    fs::remove_file(workspace.join("pyproject.toml")).unwrap();
    fs::write(
        project.join("pyproject.toml"),
        "[project]\nname = 'member'\nversion = '0.1.0'\n[tool.uv]\nmanaged = 'false'\n",
    )
    .unwrap();
    let invalid_managed = doctor(&project, &bin, &uv);
    assert_eq!(invalid_managed.status.code(), Some(1), "{invalid_managed:?}");
    assert!(text(&invalid_managed).contains("managed must be a boolean"));
    assert!(!text(&invalid_managed).contains("OK: dependencies:"));
}

#[test]
fn standalone_tool_configuration_paths_must_be_files() {
    let sandbox = tempdir().unwrap();
    let project = sandbox.path().join("project");
    let bin = sandbox.path().join("bin");
    fs::create_dir(&project).unwrap();
    let uv = install(&bin, "uv");
    install(&bin, "python");
    fs::write(
        project.join("pyproject.toml"),
        "[project]\nname = 'sample'\nversion = '0.1.0'\n",
    )
    .unwrap();

    for name in [
        "ruff.toml",
        "ty.toml",
        "pytest.ini",
        "tox.ini",
        "setup.cfg",
        "uv.toml",
    ] {
        fs::create_dir(project.join(name)).unwrap();
        let output = doctor(&project, &bin, &uv);
        let report = text(&output);
        assert_eq!(output.status.code(), Some(1), "{output:?}");
        assert!(
            report.contains(name) && report.contains("readable file"),
            "{report}"
        );
        fs::remove_dir(project.join(name)).unwrap();
    }
}

#[cfg(unix)]
#[test]
fn named_pipe_configuration_paths_are_blocking() {
    let sandbox = tempdir().unwrap();
    let project = sandbox.path().join("project");
    let bin = sandbox.path().join("bin");
    fs::create_dir(&project).unwrap();
    fs::write(
        project.join("pyproject.toml"),
        "[project]\nname = 'sample'\nversion = '0.1.0'\n",
    )
    .unwrap();
    let uv = install(&bin, "uv");
    install(&bin, "python");
    for name in [
        "ruff.toml",
        "ty.toml",
        "pytest.ini",
        "tox.ini",
        "setup.cfg",
        "uv.toml",
    ] {
        let path = project.join(name);
        assert!(Command::new("mkfifo").arg(&path).status().unwrap().success());
        let output = doctor(&project, &bin, &uv);
        assert_eq!(output.status.code(), Some(1), "{output:?}");
        assert!(text(&output).contains("readable file"), "{output:?}");
        fs::remove_file(path).unwrap();
    }
}

#[cfg(unix)]
#[test]
fn dangling_configuration_symlinks_are_blocking() {
    use std::os::unix::fs::symlink;

    let sandbox = tempdir().unwrap();
    let project = sandbox.path().join("project");
    let bin = sandbox.path().join("bin");
    fs::create_dir(&project).unwrap();
    fs::write(
        project.join("pyproject.toml"),
        "[project]\nname = 'sample'\nversion = '0.1.0'\n",
    )
    .unwrap();
    let uv = install(&bin, "uv");
    install(&bin, "python");
    for name in [
        "ruff.toml",
        "ty.toml",
        "pytest.ini",
        "tox.ini",
        "setup.cfg",
        "uv.toml",
    ] {
        let path = project.join(name);
        symlink("missing", &path).unwrap();
        let output = doctor(&project, &bin, &uv);
        assert_eq!(output.status.code(), Some(1), "{output:?}");
        assert!(text(&output).contains("cannot read"), "{output:?}");
        assert!(text(&output).contains(name), "{output:?}");
        fs::remove_file(path).unwrap();
    }
}

#[cfg(unix)]
#[test]
fn named_pipe_ancestor_metadata_is_blocking() {
    let sandbox = tempdir().unwrap();
    let project = sandbox.path().join("parent/project");
    let bin = sandbox.path().join("bin");
    fs::create_dir_all(&project).unwrap();
    fs::write(
        project.join("pyproject.toml"),
        "[project]\nname = 'sample'\nversion = '0.1.0'\n",
    )
    .unwrap();
    let ancestor = sandbox.path().join("parent/pyproject.toml");
    assert!(Command::new("mkfifo").arg(&ancestor).status().unwrap().success());
    let uv = install(&bin, "uv");
    install(&bin, "python");
    let output = doctor(&project, &bin, &uv);
    assert_eq!(output.status.code(), Some(1), "{output:?}");
    assert!(text(&output).contains("workspace metadata"), "{output:?}");
}

#[cfg(unix)]
#[test]
fn dangling_ancestor_metadata_is_blocking() {
    use std::os::unix::fs::symlink;

    let sandbox = tempdir().unwrap();
    let project = sandbox.path().join("parent/project");
    let bin = sandbox.path().join("bin");
    fs::create_dir_all(&project).unwrap();
    fs::write(
        project.join("pyproject.toml"),
        "[project]\nname = 'sample'\nversion = '0.1.0'\n",
    )
    .unwrap();
    symlink("missing", sandbox.path().join("parent/pyproject.toml")).unwrap();
    let uv = install(&bin, "uv");
    install(&bin, "python");
    let output = doctor(&project, &bin, &uv);
    let report = text(&output);
    assert_eq!(output.status.code(), Some(1), "{output:?}");
    assert!(
        report.contains("parent/pyproject.toml")
            && report.contains("dangling symlink"),
        "{report}"
    );
}

#[cfg(unix)]
#[test]
fn python_pin_must_be_a_readable_regular_file() {
    use std::os::unix::fs::symlink;

    let sandbox = tempdir().unwrap();
    let project = sandbox.path().join("project");
    let bin = sandbox.path().join("bin");
    fs::create_dir(&project).unwrap();
    fs::write(
        project.join("pyproject.toml"),
        "[project]\nname = 'sample'\nversion = '0.1.0'\n",
    )
    .unwrap();
    let uv = install(&bin, "uv");
    install(&bin, "python");
    let pin = project.join(".python-version");
    assert!(Command::new("mkfifo").arg(&pin).status().unwrap().success());
    let fifo = doctor(&project, &bin, &uv);
    assert_eq!(fifo.status.code(), Some(1), "{fifo:?}");
    assert!(text(&fifo).contains("must be a readable file"), "{fifo:?}");
    fs::remove_file(&pin).unwrap();
    symlink("missing", &pin).unwrap();
    let dangling = doctor(&project, &bin, &uv);
    assert_eq!(dangling.status.code(), Some(1), "{dangling:?}");
    assert!(text(&dangling).contains("cannot read"), "{dangling:?}");
}

#[test]
fn project_managed_setting_survives_configuration_overrides() {
    let sandbox = tempdir().unwrap();
    let project = sandbox.path().join("project");
    let bin = sandbox.path().join("bin");
    fs::create_dir(&project).unwrap();
    fs::write(
        project.join("pyproject.toml"),
        "[project]\nname = 'sample'\nversion = '0.1.0'\n[tool.uv]\nmanaged = false\n",
    )
    .unwrap();
    let uv = install(&bin, "uv");
    install(&bin, "python");
    let blocked = doctor(&project, &bin, &uv);
    assert_eq!(blocked.status.code(), Some(1), "{blocked:?}");
    assert!(text(&blocked).contains("managed = false"), "{blocked:?}");
    fs::write(project.join("uv.toml"), "offline = true\n").unwrap();
    let adjacent = doctor(&project, &bin, &uv);
    assert_eq!(adjacent.status.code(), Some(1), "{adjacent:?}");
    assert!(text(&adjacent).contains("managed = false"), "{adjacent:?}");

    fs::remove_file(project.join("uv.toml")).unwrap();
    let no_config = doctor_command(&project, &bin, &uv)
        .env("UV_NO_CONFIG", "1")
        .output()
        .unwrap();
    assert_eq!(no_config.status.code(), Some(1), "{no_config:?}");
    assert!(text(&no_config).contains("managed = false"), "{no_config:?}");

    let override_file = project.join("override.toml");
    fs::write(&override_file, "offline = true\n").unwrap();
    let explicit = doctor_command(&project, &bin, &uv)
        .env("UV_CONFIG_FILE", &override_file)
        .output()
        .unwrap();
    assert_eq!(explicit.status.code(), Some(1), "{explicit:?}");
    assert!(text(&explicit).contains("managed = false"), "{explicit:?}");
}

#[test]
fn malformed_pin_is_reported_without_a_python_executable() {
    let sandbox = tempdir().unwrap();
    let project = sandbox.path().join("project");
    let bin = sandbox.path().join("bin");
    fs::create_dir(&project).unwrap();
    fs::write(
        project.join("pyproject.toml"),
        "[project]\nname = 'sample'\nversion = '0.1.0'\n",
    )
    .unwrap();
    fs::create_dir(project.join(".python-version")).unwrap();
    let uv = install(&bin, "uv");
    let output = doctor(&project, &bin, &uv);
    let report = text(&output);
    assert_eq!(output.status.code(), Some(1), "{output:?}");
    assert!(report.contains("ERROR: Python:"), "{report}");
    assert!(
        report.contains(".python-version must be a readable file"),
        "{report}"
    );
}

#[cfg(unix)]
#[test]
fn dangling_layout_symlinks_are_blocking() {
    use std::os::unix::fs::symlink;

    let sandbox = tempdir().unwrap();
    let project = sandbox.path().join("project");
    let bin = sandbox.path().join("bin");
    fs::create_dir(&project).unwrap();
    fs::write(
        project.join("pyproject.toml"),
        "[project]\nname = 'sample'\nversion = '0.1.0'\n",
    )
    .unwrap();
    let uv = install(&bin, "uv");
    install(&bin, "python");
    for name in ["src", "tests", ".venv"] {
        let path = project.join(name);
        symlink("missing", &path).unwrap();
        let output = doctor(&project, &bin, &uv);
        assert_eq!(output.status.code(), Some(1), "{output:?}");
        assert!(text(&output).contains("must be a directory"), "{output:?}");
        fs::remove_file(path).unwrap();
    }
}

#[test]
fn invalid_project_name_is_blocking() {
    let sandbox = tempdir().unwrap();
    let project = sandbox.path().join("project");
    let bin = sandbox.path().join("bin");
    fs::create_dir(&project).unwrap();
    fs::write(project.join("pyproject.toml"), "[project]\nname = 42\n")
        .unwrap();
    let uv = install(&bin, "uv");
    install(&bin, "python");
    let output = doctor(&project, &bin, &uv);
    assert_eq!(output.status.code(), Some(1), "{output:?}");
    assert!(text(&output).contains("[project].name must be a nonempty string"));
}

#[test]
fn missing_project_metadata_and_invalid_package_are_blocking() {
    let sandbox = tempdir().unwrap();
    let project = sandbox.path().join("project");
    let bin = sandbox.path().join("bin");
    fs::create_dir(&project).unwrap();
    let uv = install(&bin, "uv");
    install(&bin, "python");
    let metadata = project.join("pyproject.toml");

    fs::write(&metadata, "[project]\nversion = '0.1.0'\n").unwrap();
    let missing_name = doctor(&project, &bin, &uv);
    assert_eq!(missing_name.status.code(), Some(1), "{missing_name:?}");
    assert!(
        text(&missing_name)
            .contains("[project].name must be a nonempty string")
    );

    fs::write(&metadata, "[project]\nname = 'sample'\n").unwrap();
    let missing_version = doctor(&project, &bin, &uv);
    assert_eq!(missing_version.status.code(), Some(1), "{missing_version:?}");
    assert!(text(&missing_version).contains("[project].version is required"));

    fs::write(&metadata, "[project]\nname = 'sample'\ndynamic = ['version']\n")
        .unwrap();
    let dynamic_version = doctor(&project, &bin, &uv);
    assert_eq!(dynamic_version.status.code(), Some(0), "{dynamic_version:?}");

    fs::write(&metadata, "[project]\nname = 'sample'\nversion = '0.1.0'\n[tool.uv]\npackage = 'invalid'\n").unwrap();
    fs::write(project.join("uv.toml"), "offline = true\n").unwrap();
    for no_config in [false, true] {
        let mut command = doctor_command(&project, &bin, &uv);
        if no_config {
            command.env("UV_NO_CONFIG", "1");
        }
        let invalid_package = command.output().unwrap();
        assert_eq!(
            invalid_package.status.code(),
            Some(1),
            "{invalid_package:?}"
        );
        assert!(
            text(&invalid_package)
                .contains("[tool.uv] package must be a boolean")
        );
    }
}

#[test]
fn real_uv_rejects_unknown_standalone_settings() {
    let sandbox = tempdir().unwrap();
    let project = sandbox.path().join("project");
    let bin = sandbox.path().join("bin");
    fs::create_dir(&project).unwrap();
    fs::write(
        project.join("pyproject.toml"),
        "[project]\nname = 'sample'\nversion = '0.1.0'\n",
    )
    .unwrap();
    fs::write(project.join("uv.toml"), "nonsense = true\n").unwrap();
    install(&bin, "python");
    let uv = which::which("uv").unwrap();
    let output = doctor(&project, &bin, &uv);
    assert_eq!(output.status.code(), Some(1), "{output:?}");
    assert!(text(&output).contains("uv rejected"), "{output:?}");
    assert!(text(&output).contains("uv.toml"), "{output:?}");

    let cache = project.join("uv-cache");
    fs::write(
        project.join("uv.toml"),
        format!("cache-dir = '{}'\n", cache.display()),
    )
    .unwrap();
    let valid = doctor_command(&project, &bin, &uv)
        .env("UV_WORKING_DIR", project.join("missing-working-dir"))
        .output()
        .unwrap();
    assert_eq!(valid.status.code(), Some(0), "{valid:?}");
    assert!(!cache.exists());

    fs::remove_file(project.join("uv.toml")).unwrap();
    fs::write(
        project.join("pyproject.toml"),
        "[project]\nname = 'sample'\nversion = '0.1.0'\n[tool.uv]\ndefualt-groups = ['dev']\n",
    )
    .unwrap();
    let invalid_project_settings = doctor(&project, &bin, &uv);
    assert_eq!(
        invalid_project_settings.status.code(),
        Some(1),
        "{invalid_project_settings:?}"
    );
    assert!(
        text(&invalid_project_settings).contains("uv rejected"),
        "{invalid_project_settings:?}"
    );
    assert!(
        text(&invalid_project_settings).contains("defualt-groups"),
        "{invalid_project_settings:?}"
    );

    fs::write(project.join("pyproject.toml"), "[project]\nname = 'sample'\nversion = '0.1.0'\n[dependency-groups]\ndev = []\n[tool.uv]\ndefault-groups = ['dev']\n").unwrap();
    let user_file = sandbox.path().join(".doctor-user-config/uv/uv.toml");
    fs::create_dir_all(user_file.parent().unwrap()).unwrap();
    fs::write(&user_file, "nonsense = true\n").unwrap();
    let invalid_user_settings = doctor(&project, &bin, &uv);
    let report = text(&invalid_user_settings);
    assert_eq!(
        invalid_user_settings.status.code(),
        Some(1),
        "{invalid_user_settings:?}"
    );
    assert!(
        report.contains("uv rejected")
            && report.contains(".doctor-user-config"),
        "{report}"
    );
    assert!(
        !report.contains(&format!(
            "uv rejected {}",
            project.join("pyproject.toml").display()
        )),
        "{report}"
    );
}

#[test]
fn workspace_ancestor_only_blocks_matching_nonexcluded_members() {
    let sandbox = tempdir().unwrap();
    let workspace = sandbox.path().join("workspace");
    let bin = sandbox.path().join("bin");
    fs::create_dir_all(workspace.join("packages/member")).unwrap();
    fs::create_dir_all(workspace.join("packages/member/standalone")).unwrap();
    fs::create_dir_all(workspace.join("packages/excluded")).unwrap();
    fs::create_dir_all(workspace.join("other")).unwrap();
    let uv = install(&bin, "uv");
    install(&bin, "python");
    fs::write(workspace.join("pyproject.toml"), "[project]\nname = 'workspace'\nversion = '0.1.0'\n[tool.uv.workspace]\nmembers = ['packages/*']\nexclude = ['packages/ex*']\n").unwrap();
    for relative in [
        "packages/member",
        "packages/member/standalone",
        "packages/excluded",
        "other",
    ] {
        let project = workspace.join(relative);
        fs::write(
            project.join("pyproject.toml"),
            "[project]\nname = 'sample'\nversion = '0.1.0'\n[dependency-groups]\ndev = []\n",
        )
        .unwrap();
        let output = doctor(&project, &bin, &uv);
        let report = text(&output);
        let member = relative == "packages/member";
        assert_eq!(
            output.status.code(),
            Some(if member { 1 } else { 0 }),
            "{output:?}"
        );
        assert_eq!(report.contains("uv workspace"), member, "{report}");
        assert_eq!(report.contains("OK: dependencies:"), !member, "{report}");
    }
}

#[test]
fn workspace_globs_normalize_dot_and_match_parent_directories() {
    let sandbox = tempdir().unwrap();
    let workspace = sandbox.path().join("workspace");
    let project = workspace.join("packages/group/member");
    let bin = sandbox.path().join("bin");
    fs::create_dir_all(&project).unwrap();
    let uv = install(&bin, "uv");
    install(&bin, "python");
    fs::write(
        project.join("pyproject.toml"),
        "[project]\nname = 'member'\nversion = '0.1.0'\n",
    )
    .unwrap();
    fs::write(
        workspace.join("pyproject.toml"),
        "[tool.uv.workspace]\nmembers = ['./packages/*/member']\n",
    )
    .unwrap();
    let included = doctor(&project, &bin, &uv);
    assert_eq!(included.status.code(), Some(1), "{included:?}");
    assert!(text(&included).contains("uv workspace"));

    fs::write(workspace.join("pyproject.toml"), "[tool.uv.workspace]\nmembers = ['./packages/*/member']\nexclude = ['./packages/*']\n").unwrap();
    let excluded = doctor(&project, &bin, &uv);
    assert_eq!(excluded.status.code(), Some(0), "{excluded:?}");
    assert!(!text(&excluded).contains("uv workspace"));
}

#[test]
fn nearest_project_or_nonmatching_workspace_stops_workspace_discovery() {
    let sandbox = tempdir().unwrap();
    let outer = sandbox.path().join("outer");
    let middle = outer.join("middle");
    let project = middle.join("member");
    let bin = sandbox.path().join("bin");
    fs::create_dir_all(&project).unwrap();
    let uv = install(&bin, "uv");
    install(&bin, "python");
    fs::write(
        outer.join("pyproject.toml"),
        "[tool.uv.workspace]\nmembers = ['**']\n",
    )
    .unwrap();
    fs::write(
        project.join("pyproject.toml"),
        "[project]\nname = 'member'\nversion = '0.1.0'\n",
    )
    .unwrap();

    fs::write(
        middle.join("pyproject.toml"),
        "[project]\nname = 'middle'\nversion = '0.1.0'\n",
    )
    .unwrap();
    let intervening = doctor(&project, &bin, &uv);
    assert_eq!(intervening.status.code(), Some(0), "{intervening:?}");
    assert!(!text(&intervening).contains("uv workspace"));

    fs::write(
        middle.join("pyproject.toml"),
        "[tool.uv.workspace]\nmembers = ['other']\n",
    )
    .unwrap();
    let nonmember = doctor(&project, &bin, &uv);
    assert_eq!(nonmember.status.code(), Some(0), "{nonmember:?}");
    assert!(!text(&nonmember).contains("uv workspace"));
}

#[test]
fn invalid_workspace_glob_is_blocking() {
    let sandbox = tempdir().unwrap();
    let workspace = sandbox.path().join("workspace");
    let project = workspace.join("member");
    let bin = sandbox.path().join("bin");
    fs::create_dir_all(&project).unwrap();
    let uv = install(&bin, "uv");
    install(&bin, "python");
    fs::write(
        project.join("pyproject.toml"),
        "[project]\nname = 'member'\nversion = '0.1.0'\n",
    )
    .unwrap();
    fs::write(
        workspace.join("pyproject.toml"),
        "[tool.uv.workspace]\nmembers = ['[']\n",
    )
    .unwrap();
    let output = doctor(&project, &bin, &uv);
    assert_eq!(output.status.code(), Some(1), "{output:?}");
    assert!(text(&output).contains("invalid workspace glob"));
    assert!(!text(&output).contains("OK: dependencies:"));

    fs::write(
        workspace.join("pyproject.toml"),
        "[tool.uv.workspace]\nmembers = ['other']\nexclude = ['[']\n",
    )
    .unwrap();
    let unmatched = doctor(&project, &bin, &uv);
    assert_eq!(unmatched.status.code(), Some(0), "{unmatched:?}");
    assert!(!text(&unmatched).contains("invalid workspace glob"));

    fs::write(
        workspace.join("pyproject.toml"),
        "[tool.uv.workspace]\nmembers = ['member']\nexclude = ['[']\n",
    )
    .unwrap();
    let excluded = doctor(&project, &bin, &uv);
    assert_eq!(excluded.status.code(), Some(1), "{excluded:?}");
    assert!(text(&excluded).contains("invalid workspace glob"));
}

#[test]
fn unmanaged_project_does_not_inherit_an_ancestor_workspace() {
    let sandbox = tempdir().unwrap();
    let workspace = sandbox.path().join("workspace");
    let project = workspace.join("member");
    let bin = sandbox.path().join("bin");
    fs::create_dir_all(&project).unwrap();
    let uv = install(&bin, "uv");
    install(&bin, "python");
    fs::write(
        workspace.join("pyproject.toml"),
        "[tool.uv.workspace]\nmembers = ['member']\n",
    )
    .unwrap();
    fs::write(
        project.join("pyproject.toml"),
        "[project]\nname = 'member'\nversion = '0.1.0'\n[tool.uv]\nmanaged = false\n",
    )
    .unwrap();

    let output = doctor(&project, &bin, &uv);
    let report = text(&output);
    assert_eq!(output.status.code(), Some(1), "{output:?}");
    assert!(report.contains("managed = false"), "{report}");
    assert!(!report.contains("uv workspace"), "{report}");
    assert!(!report.contains("OK: dependencies:"), "{report}");
}

#[cfg(target_os = "linux")]
#[test]
fn non_utf8_workspace_path_cannot_be_classified_silently() {
    use std::os::unix::ffi::OsStringExt;
    let sandbox = tempdir().unwrap();
    let workspace = sandbox.path().join("workspace");
    let project =
        workspace.join(std::ffi::OsString::from_vec(b"member-\xff".to_vec()));
    let bin = sandbox.path().join("bin");
    fs::create_dir_all(&project).unwrap();
    let uv = install(&bin, "uv");
    install(&bin, "python");
    fs::write(
        project.join("pyproject.toml"),
        "[project]\nname = 'member'\nversion = '0.1.0'\n",
    )
    .unwrap();
    fs::write(
        workspace.join("pyproject.toml"),
        "[tool.uv.workspace]\nmembers = ['member-*']\n",
    )
    .unwrap();
    let output = doctor(&project, &bin, &uv);
    assert_eq!(output.status.code(), Some(1), "{output:?}");
    assert!(text(&output).contains("cannot determine uv workspace membership"));
    assert!(!text(&output).contains("OK: dependencies:"));
}

#[test]
fn user_uv_configuration_accepts_valid_settings_and_reports_invalid_fields() {
    let sandbox = tempdir().unwrap();
    let project = sandbox.path().join("project");
    let bin = sandbox.path().join("bin");
    fs::create_dir(&project).unwrap();
    let uv = install(&bin, "uv");
    install(&bin, "python");
    fs::write(project.join("pyproject.toml"), "[project]\nname = 'sample'\nversion = '0.1.0'\n[dependency-groups]\ndev = []\nquality = []\n").unwrap();
    let user_file = sandbox.path().join(".doctor-user-config/uv/uv.toml");
    fs::create_dir_all(user_file.parent().unwrap()).unwrap();
    fs::write(&user_file, "offline = true\n").unwrap();

    let output = doctor(&project, &bin, &uv);
    let report = text(&output);
    assert_eq!(output.status.code(), Some(0), "{output:?}");
    assert!(report.contains("group \"quality\" is not checked"), "{report}");
    assert!(!report.contains("group \"dev\" is not checked"), "{report}");

    fs::write(&user_file, "dev-dependencies = [42]\n").unwrap();
    let malformed = doctor(&project, &bin, &uv);
    let report = text(&malformed);
    assert_eq!(malformed.status.code(), Some(1), "{malformed:?}");
    assert!(
        report.contains(".doctor-user-config") && report.contains("uv.toml"),
        "{report}"
    );
    assert!(
        report.contains(
            "dev-dependencies is only allowed in [tool.uv] in pyproject.toml"
        ),
        "{report}"
    );
    assert!(!report.contains("OK: dependencies:"), "{report}");
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
    fs::write(project.join("pyproject.toml"), "[project]\nname = 'sample'\nversion = '0.1.0'\n[dependency-groups]\ndev = []\nquality = []\n").unwrap();
    let home = sandbox.path().join("home");
    let user_file = home.join(".config/uv/uv.toml");
    fs::create_dir_all(user_file.parent().unwrap()).unwrap();
    fs::write(&user_file, "offline = true\n").unwrap();

    let output = doctor_command(&project, &bin, &uv)
        .env_remove("XDG_CONFIG_HOME")
        .env("HOME", &home)
        .output()
        .unwrap();
    let report = text(&output);
    assert_eq!(output.status.code(), Some(0), "{output:?}");
    assert!(report.contains("group \"quality\" is not checked"), "{report}");
    assert!(!report.contains("group \"dev\" is not checked"), "{report}");
}

#[test]
fn project_only_uv_fields_are_rejected_in_all_configuration_locations() {
    let sandbox = tempdir().unwrap();
    let project = sandbox.path().join("project");
    let bin = sandbox.path().join("bin");
    fs::create_dir(&project).unwrap();
    let uv = install(&bin, "uv");
    install(&bin, "python");
    fs::write(
        project.join("pyproject.toml"),
        "[project]\nname = 'sample'\nversion = '0.1.0'\n[dependency-groups]\ndev = []\n",
    )
    .unwrap();
    let project_file = project.join("uv.toml");
    let user_file = sandbox.path().join(".doctor-user-config/uv/uv.toml");
    fs::create_dir_all(user_file.parent().unwrap()).unwrap();
    let system_file = sandbox.path().join(".doctor-system-config/uv/uv.toml");
    for field in [
        "conflicts",
        "workspace",
        "sources",
        "dev-dependencies",
        "default-groups",
        "dependency-groups",
        "managed",
        "package",
        "build-backend",
        "environments",
        "required-environments",
    ] {
        fs::write(&project_file, format!("{field} = true\n")).unwrap();
        let output = doctor(&project, &bin, &uv);
        let report = text(&output);
        assert_eq!(output.status.code(), Some(1), "{output:?}");
        assert!(
            report.contains(&format!("{field} is only allowed")),
            "{report}"
        );
        if field == "dependency-groups" {
            assert!(report.contains("[tool.uv.dependency-groups]"), "{report}");
        }
        assert!(
            report.contains("project") && report.contains("uv.toml"),
            "{report}"
        );
        assert!(!report.contains("OK: dependencies:"), "{report}");
    }
    fs::remove_file(&project_file).unwrap();

    for (path, field) in
        [(&user_file, "default-groups"), (&system_file, "dependency-groups")]
    {
        fs::write(path, format!("{field} = true\n")).unwrap();
        let output = doctor(&project, &bin, &uv);
        let report = text(&output);
        assert_eq!(output.status.code(), Some(1), "{output:?}");
        assert!(
            report.contains(&format!("{field} is only allowed")),
            "{report}"
        );
        let configuration_scope = if field == "default-groups" {
            ".doctor-user-config"
        } else {
            ".doctor-system-config"
        };
        assert!(
            report.contains(configuration_scope) && report.contains("uv.toml"),
            "{report}"
        );
        assert!(!report.contains("OK: dependencies:"), "{report}");
        fs::remove_file(path).unwrap();
    }

    let explicit = project.join("override.toml");
    fs::write(&explicit, "dev-dependencies = []\n").unwrap();
    let output = doctor_command(&project, &bin, &uv)
        .env("UV_CONFIG_FILE", &explicit)
        .output()
        .unwrap();
    let report = text(&output);
    assert_eq!(output.status.code(), Some(1), "{output:?}");
    assert!(report.contains("dev-dependencies is only allowed"), "{report}");
    assert!(report.contains("override.toml"), "{report}");
    assert!(report.contains("unset UV_CONFIG_FILE"), "{report}");
}

#[test]
fn project_pyproject_defaults_are_unaffected_by_valid_user_settings() {
    let sandbox = tempdir().unwrap();
    let project = sandbox.path().join("project");
    let bin = sandbox.path().join("bin");
    fs::create_dir(&project).unwrap();
    let uv = install(&bin, "uv");
    install(&bin, "python");
    fs::write(
        project.join("pyproject.toml"),
        "[project]\nname = 'sample'\nversion = '0.1.0'\n[dependency-groups]\nqa = []\ndev = []\n[tool.uv]\ndefault-groups = ['qa']\n",
    )
    .unwrap();
    let user_file = sandbox.path().join(".doctor-user-config/uv/uv.toml");
    fs::create_dir_all(user_file.parent().unwrap()).unwrap();
    fs::write(&user_file, "offline = true\n").unwrap();

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
        "[project]\nname = 'sample'\nversion = '0.1.0'\n[dependency-groups]\ndev = []\n",
    )
    .unwrap();
    fs::write(project.join("uv.toml"), "offline = true\n").unwrap();
    let user_file = sandbox.path().join(".doctor-user-config/uv/uv.toml");
    fs::create_dir_all(user_file.parent().unwrap()).unwrap();
    fs::write(&user_file, "offline = [\n").unwrap();

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

#[test]
fn malformed_and_unreadable_system_configuration_are_blocking() {
    let sandbox = tempdir().unwrap();
    let project = sandbox.path().join("project");
    let bin = sandbox.path().join("bin");
    fs::create_dir(&project).unwrap();
    let uv = install(&bin, "uv");
    install(&bin, "python");
    fs::write(
        project.join("pyproject.toml"),
        "[project]\nname = 'sample'\nversion = '0.1.0'\n[dependency-groups]\ndev = []\n",
    )
    .unwrap();
    let system_file = sandbox.path().join(".doctor-system-config/uv/uv.toml");
    fs::create_dir_all(system_file.parent().unwrap()).unwrap();
    fs::write(&system_file, "offline = [\n").unwrap();

    let malformed = doctor(&project, &bin, &uv);
    let report = text(&malformed);
    assert_eq!(malformed.status.code(), Some(1), "{malformed:?}");
    assert!(
        report.contains(".doctor-system-config") && report.contains("uv.toml"),
        "{report}"
    );
    assert!(report.contains("invalid"), "{report}");
    assert!(!report.contains("OK: dependencies:"), "{report}");

    fs::remove_file(&system_file).unwrap();
    fs::create_dir(&system_file).unwrap();
    let unreadable = doctor(&project, &bin, &uv);
    let report = text(&unreadable);
    assert_eq!(unreadable.status.code(), Some(1), "{unreadable:?}");
    assert!(
        report.contains(".doctor-system-config") && report.contains("uv.toml"),
        "{report}"
    );
    assert!(report.contains("cannot read"), "{report}");
    assert!(!report.contains("OK: dependencies:"), "{report}");
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
    fs::write(project.join("pyproject.toml"), "[project]\nname = 'sample'\nversion = '0.1.0'\n[dependency-groups]\ndev = []\nquality = []\n").unwrap();
    let first = sandbox.path().join("first/uv/uv.toml");
    let second = sandbox.path().join("second/uv/uv.toml");
    fs::create_dir_all(first.parent().unwrap()).unwrap();
    fs::create_dir_all(second.parent().unwrap()).unwrap();
    fs::write(&first, "offline = true\n").unwrap();
    fs::write(&second, "default-groups = ['quality']\n").unwrap();
    let paths = std::env::join_paths([
        first.parent().unwrap().parent().unwrap(),
        second.parent().unwrap().parent().unwrap(),
    ])
    .unwrap();

    let output = doctor_command(&project, &bin, &uv)
        .env("XDG_CONFIG_DIRS", &paths)
        .output()
        .unwrap();
    let report = text(&output);
    assert_eq!(output.status.code(), Some(0), "{output:?}");
    assert!(report.contains("group \"quality\" is not checked"), "{report}");
    assert!(!report.contains("group \"dev\" is not checked"), "{report}");

    fs::write(&first, "default-groups = ['quality']\n").unwrap();
    let invalid = doctor_command(&project, &bin, &uv)
        .env("XDG_CONFIG_DIRS", &paths)
        .output()
        .unwrap();
    let report = text(&invalid);
    assert_eq!(invalid.status.code(), Some(1), "{invalid:?}");
    assert!(report.contains("first") && report.contains("uv.toml"), "{report}");
    assert!(!report.contains("second"), "{report}");
}

fn doctor(project: &Path, path: &Path, uv: &Path) -> Output {
    doctor_command(project, path, uv).output().unwrap()
}

fn text(output: &Output) -> String {
    String::from_utf8(output.stdout.clone()).unwrap()
}

#[test]
fn project_only_uv_settings_in_uv_toml_are_blocking() {
    let sandbox = tempdir().unwrap();
    let project = sandbox.path().join("project");
    let bin = sandbox.path().join("bin");
    fs::create_dir(&project).unwrap();
    let uv = install(&bin, "uv");
    install(&bin, "python");
    fs::write(
        project.join("pyproject.toml"),
        "[project]\nname = 'sample'\nversion = '0.1.0'\n[dependency-groups]\ndev = []\n",
    )
    .unwrap();
    let path = project.join("uv.toml");
    fs::write(&path, "default-groups = ['dev']\n").unwrap();

    let output = doctor(&project, &bin, &uv);
    let report = text(&output);
    assert_eq!(output.status.code(), Some(1), "{output:?}");
    assert!(
        report.contains(
            "default-groups is only allowed in [tool.uv] in pyproject.toml"
        ),
        "{report}"
    );
    assert!(report.contains("uv.toml"), "{report}");
    assert!(!report.contains("OK: dependencies:"), "{report}");
}

#[test]
fn healthy_fixture_reports_all_versions_without_writing_project() {
    let sandbox = tempdir().unwrap();
    let project = sandbox.path().join("project");
    let bin = sandbox.path().join("bin");
    fs::create_dir(&project).unwrap();
    let metadata =
        "[project]\nname = 'sample'\nversion = '0.1.0'\ndependencies = []\n";
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
    fs::write(
        project.join("pyproject.toml"),
        "[project]\nname = 'sample'\nversion = '0.1.0'\n",
    )
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
        "[project]\nname = 'sample'\nversion = '0.1.0'\n[tool.pyfaro]\nschema-version = 2\n",
    )
    .unwrap();
    fs::write(project.join("src"), "wrong kind").unwrap();
    let invalid = doctor(&project, &bin, &uv);
    assert_eq!(invalid.status.code(), Some(1), "{invalid:?}");
    assert!(text(&invalid).contains("schema-version"));
    assert!(text(&invalid).contains("src must be a directory"), "{invalid:?}");

    fs::write(
        &metadata,
        "[project]\nname = 'sample'\nversion = '0.1.0'\n[tool.ruff]\nline-length = 88\n",
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
        "[project]\nname = 'sample'\nversion = '0.1.0'\nrequires-python = '>=3.13'\n",
    )
    .unwrap();
    let incompatible = doctor(&project, &bin, &uv);
    assert_eq!(incompatible.status.code(), Some(1), "{incompatible:?}");
    assert!(text(&incompatible).contains("does not satisfy requires-python"));

    for dependencies in ["'invalid'", "[42]"] {
        fs::write(
            &metadata,
            format!(
                "[project]\nname = 'sample'\nversion = '0.1.0'\ndependencies = {dependencies}\n"
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
        "[project]\nname = 'sample'\nversion = '0.1.0'\ndynamic = ['dependencies']\n",
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
        "[project]\nname = 'sample'\nversion = '0.1.0'\ndependencies = []\ndynamic = ['dependencies']\n",
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
        "[project]\nname = 'sample'\nversion = '0.1.0'\n[tool]\nuv = 'invalid'\n",
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
    fs::write(project.join("uv.toml"), "offline = true\n").unwrap();
    let malformed_ignored_uv = doctor(&project, &bin, &uv);
    assert_eq!(
        malformed_ignored_uv.status.code(),
        Some(1),
        "{malformed_ignored_uv:?}"
    );
    assert!(text(&malformed_ignored_uv).contains("must be a table"));
    fs::remove_file(project.join("uv.toml")).unwrap();
    let no_config = doctor_command(&project, &bin, &uv)
        .env("UV_NO_CONFIG", "1")
        .output()
        .unwrap();
    assert_eq!(no_config.status.code(), Some(1), "{no_config:?}");
    assert!(text(&no_config).contains("must be a table"));

    fs::write(&metadata, "[project]\nname = 'sample'\nversion = '0.1.0'\n")
        .unwrap();
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
        "[project]\nname = 'sample'\nversion = '0.1.0'\ndependencies = []\n",
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
    fs::write(&metadata, "[project]\nname = 'sample'\nversion = '0.1.0'\n[dependency-groups]\nQA__Test = [{include-group = 'other..test'}]\nOther-Test = []\n[tool.uv]\ndefault-groups = ['qa.test']\n").unwrap();
    let output = doctor(&project, &bin, &uv);
    assert_eq!(output.status.code(), Some(0), "{output:?}");
    let report = text(&output);
    assert!(!report.contains("is not checked"), "{report}");
    assert!(report.contains("no blocking issues"), "{report}");
}

#[test]
fn real_uv_accepts_standalone_dependency_policy_fields() {
    let sandbox = tempdir().unwrap();
    let project = sandbox.path().join("project");
    let bin = sandbox.path().join("bin");
    fs::create_dir(&project).unwrap();
    fs::write(
        project.join("pyproject.toml"),
        "[project]\nname = 'sample'\nversion = '0.1.0'\n",
    )
    .unwrap();
    install(&bin, "python");
    let uv = which::which("uv").unwrap();
    for field in [
        "build-constraint-dependencies",
        "constraint-dependencies",
        "exclude-dependencies",
        "override-dependencies",
    ] {
        fs::write(project.join("uv.toml"), format!("{field} = []\n")).unwrap();
        let output = doctor(&project, &bin, &uv);
        assert_eq!(output.status.code(), Some(0), "{field}: {output:?}");
    }
}

#[test]
fn real_uv_checks_only_effective_required_version() {
    let sandbox = tempdir().unwrap();
    let project = sandbox.path().join("project");
    let bin = sandbox.path().join("bin");
    fs::create_dir(&project).unwrap();
    fs::write(
        project.join("pyproject.toml"),
        "[project]\nname = 'sample'\nversion = '0.1.0'\n",
    )
    .unwrap();
    install(&bin, "python");
    let uv = which::which("uv").unwrap();
    let user_file = sandbox.path().join(".doctor-user-config/uv/uv.toml");
    fs::create_dir_all(user_file.parent().unwrap()).unwrap();
    fs::write(&user_file, "required-version = '<0.9'\n").unwrap();
    fs::write(project.join("uv.toml"), "required-version = '>=0.9.26'\n")
        .unwrap();

    let overridden = doctor(&project, &bin, &uv);
    assert_eq!(overridden.status.code(), Some(0), "{overridden:?}");
    fs::remove_file(project.join("uv.toml")).unwrap();
    let effective = doctor(&project, &bin, &uv);
    assert_eq!(effective.status.code(), Some(1), "{effective:?}");
    assert!(text(&effective).contains("uv rejected effective configuration"));
}

#[test]
fn project_uv_toml_preserves_project_dependency_metadata() {
    let sandbox = tempdir().unwrap();
    let project = sandbox.path().join("project");
    let bin = sandbox.path().join("bin");
    fs::create_dir(&project).unwrap();
    let uv = install(&bin, "uv");
    install(&bin, "python");
    fs::write(project.join("pyproject.toml"), "[project]\nname = 'sample'\nversion = '0.1.0'\n[dependency-groups]\ndev = []\nquality = []\n[tool.uv]\ndefault-groups = ['quality']\ndev-dependencies = []\n").unwrap();
    fs::write(project.join("uv.toml"), "offline = true\n").unwrap();

    let output = doctor(&project, &bin, &uv);
    let report = text(&output);
    assert_eq!(output.status.code(), Some(0), "{output:?}");
    assert!(!report.contains("ERROR: configuration:"), "{report}");
    assert!(report.contains("group \"dev\" is not checked"), "{report}");
    assert!(!report.contains("group \"quality\" is not checked"), "{report}");
    fs::write(project.join("pyproject.toml"), "[project]\nname = 'sample'\nversion = '0.1.0'\n[dependency-groups]\ndev = []\nquality = []\n[tool.uv]\ndefault-groups = ['quality']\ndev-dependencies = [42]\n").unwrap();
    let invalid_legacy = doctor(&project, &bin, &uv);
    assert_eq!(invalid_legacy.status.code(), Some(1), "{invalid_legacy:?}");
    assert!(
        text(&invalid_legacy).contains("[tool.uv].dev-dependencies")
            && text(&invalid_legacy)
                .contains("must contain only requirement strings"),
        "{invalid_legacy:?}"
    );
}

#[test]
fn explicit_and_no_config_override_discovery_but_not_project_metadata() {
    let sandbox = tempdir().unwrap();
    let project = sandbox.path().join("project");
    let bin = sandbox.path().join("bin");
    fs::create_dir(&project).unwrap();
    let uv = install(&bin, "uv");
    install(&bin, "python");
    fs::write(project.join("pyproject.toml"), "[project]\nname = 'sample'\nversion = '0.1.0'\n[dependency-groups]\ndev = []\nquality = []\n[tool.uv]\ndefault-groups = ['quality']\n").unwrap();
    fs::write(project.join("uv.toml"), "offline = true\n").unwrap();
    fs::write(project.join("override.toml"), "offline = false\n").unwrap();
    let user_file = sandbox.path().join(".doctor-user-config/uv/uv.toml");
    fs::create_dir_all(user_file.parent().unwrap()).unwrap();
    fs::write(&user_file, "offline = [\n").unwrap();

    let discovered = doctor(&project, &bin, &uv);
    assert_eq!(discovered.status.code(), Some(1), "{discovered:?}");
    let report = text(&discovered);
    assert!(
        report.contains(".doctor-user-config") && report.contains("uv.toml"),
        "{report}"
    );

    let override_output = doctor_command(&project, &bin, &uv)
        .current_dir(sandbox.path())
        .env("UV_CONFIG_FILE", "override.toml")
        .output()
        .unwrap();
    let report = text(&override_output);
    assert_eq!(override_output.status.code(), Some(0), "{override_output:?}");
    assert!(report.contains("group \"dev\" is not checked"), "{report}");
    assert!(!report.contains("group \"quality\" is not checked"), "{report}");
    assert!(!report.contains("uv.toml takes precedence"), "{report}");

    let no_config = doctor_command(&project, &bin, &uv)
        .env("UV_NO_CONFIG", "1")
        .output()
        .unwrap();
    let report = text(&no_config);
    assert_eq!(no_config.status.code(), Some(0), "{no_config:?}");
    assert!(report.contains("group \"dev\" is not checked"), "{report}");
    assert!(!report.contains("group \"quality\" is not checked"), "{report}");
    assert!(!report.contains("uv.toml takes precedence"), "{report}");

    let both = doctor_command(&project, &bin, &uv)
        .current_dir(sandbox.path())
        .env("UV_CONFIG_FILE", "override.toml")
        .env("UV_NO_CONFIG", "1")
        .output()
        .unwrap();
    assert_eq!(both.status.code(), Some(0), "{both:?}");
    assert!(text(&both).contains("group \"dev\" is not checked"), "{both:?}");
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
        "[project]\nname = 'sample'\nversion = '0.1.0'\ndependencies = []\n",
    )
    .unwrap();

    let missing = project.join("missing.toml");
    let missing_output = doctor_command(&project, &bin, &uv)
        .env("UV_CONFIG_FILE", &missing)
        .output()
        .unwrap();
    let report = text(&missing_output);
    assert_eq!(missing_output.status.code(), Some(1), "{missing_output:?}");
    assert!(report.contains("missing.toml"), "{report}");
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
        report.contains("invalid") && report.contains("malformed.toml"),
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
    assert!(report.contains("directory.toml"), "{report}");
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
    fs::write(project.join("pyproject.toml"), "[project]\nname = 'sample'\nversion = '0.1.0'\n[tool.pytest.ini_options]\naddopts = '-q'\n").unwrap();
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
        "[project]\nname = 'sample'\nversion = '0.1.0'\n[tool.pytest.other]\nvalue = 1\n",
    )
    .unwrap();
    let nested = doctor(&project, &bin, &uv);
    assert_eq!(nested.status.code(), Some(1), "{nested:?}");
    assert!(text(&nested).contains("[tool.pytest] and .pytest.ini"));

    fs::write(project.join("pyproject.toml"), "[project]\nname = 'sample'\nversion = '0.1.0'\n[tool.pytest.other]\nvalue = 1\n[tool.pytest.ini_options]\naddopts = '-q'\n").unwrap();
    let mixed_nested = doctor(&project, &bin, &uv);
    assert_eq!(mixed_nested.status.code(), Some(1), "{mixed_nested:?}");
    assert!(
        text(&mixed_nested)
            .contains("[tool.pytest] and [tool.pytest.ini_options]")
    );

    fs::write(
        project.join("pyproject.toml"),
        "[project]\nname = 'sample'\nversion = '0.1.0'\n[tool.pytest]\naddopts = ['-q']\n",
    )
    .unwrap();
    let native_toml = doctor(&project, &bin, &uv);
    assert_eq!(native_toml.status.code(), Some(1), "{native_toml:?}");
    assert!(text(&native_toml).contains("[tool.pytest] and .pytest.ini"));
    fs::remove_file(project.join(".pytest.ini")).unwrap();

    fs::write(project.join("pyproject.toml"), "[project]\nname = 'sample'\nversion = '0.1.0'\n[tool.pytest]\naddopts = ['-q']\n[tool.pytest.ini_options]\naddopts = '-q'\n").unwrap();
    let mixed_formats = doctor(&project, &bin, &uv);
    assert_eq!(mixed_formats.status.code(), Some(1), "{mixed_formats:?}");
    assert!(
        text(&mixed_formats)
            .contains("[tool.pytest] and [tool.pytest.ini_options]")
    );

    fs::write(project.join("pyproject.toml"), "[project]\nname = 'sample'\nversion = '0.1.0'\n[tool.pytest.ini_options]\naddopts = '-q'\n").unwrap();
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

    fs::write(
        project.join("pyproject.toml"),
        "[project]\nname = 'sample'\nversion = '0.1.0'\n",
    )
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
fn pytest_nine_only_configuration_requires_pytest_nine() {
    let sandbox = tempdir().unwrap();
    let project = sandbox.path().join("project");
    let bin = sandbox.path().join("bin");
    fs::create_dir(&project).unwrap();
    let uv = install(&bin, "uv");
    install(&bin, "python");
    install(&bin, "pytest");

    for config in ["pytest.toml", ".pytest.toml", "native"] {
        fs::write(
            project.join("pyproject.toml"),
            if config == "native" {
                "[project]\nname = 'sample'\nversion = '0.1.0'\n[tool.pytest]\naddopts = ['-q']\n"
            } else {
                "[project]\nname = 'sample'\nversion = '0.1.0'\n"
            },
        )
        .unwrap();
        if config != "native" {
            fs::write(project.join(config), "").unwrap();
        }
        let old = doctor(&project, &bin, &uv);
        let report = text(&old);
        assert_eq!(old.status.code(), Some(1), "{old:?}");
        assert!(
            report.contains("pytest >=9")
                && report.contains("INI configuration"),
            "{report}"
        );

        let recent = doctor_command(&project, &bin, &uv)
            .env("PYFARO_FIXTURE_PYTEST_VERSION", "pytest 9.0.0")
            .output()
            .unwrap();
        assert_eq!(recent.status.code(), Some(0), "{recent:?}");
        assert!(!text(&recent).contains("pytest >=9"));
        if config != "native" {
            fs::remove_file(project.join(config)).unwrap();
        }
    }
}

#[test]
fn pytest_ini_configuration_remains_compatible_with_pytest_eight() {
    let sandbox = tempdir().unwrap();
    let project = sandbox.path().join("project");
    let bin = sandbox.path().join("bin");
    fs::create_dir(&project).unwrap();
    let uv = install(&bin, "uv");
    install(&bin, "python");
    install(&bin, "pytest");
    fs::write(project.join("pyproject.toml"), "[project]\nname = 'sample'\nversion = '0.1.0'\n[tool.pytest.ini_options]\naddopts = '-q'\n").unwrap();
    let output = doctor(&project, &bin, &uv);
    assert_eq!(output.status.code(), Some(0), "{output:?}");
}

#[test]
fn ancestor_uv_configuration_does_not_override_project_default_groups() {
    let sandbox = tempdir().unwrap();
    let parent = sandbox.path().join("parent");
    let project = parent.join("project");
    let bin = sandbox.path().join("bin");
    fs::create_dir_all(&project).unwrap();
    let uv = install(&bin, "uv");
    install(&bin, "python");
    fs::write(project.join("pyproject.toml"), "[project]\nname = 'sample'\nversion = '0.1.0'\n[dependency-groups]\ndev = []\nquality = []\n").unwrap();
    fs::write(
        parent.join("pyproject.toml"),
        "[tool.uv]\ndefault-groups = ['dev']\n",
    )
    .unwrap();
    fs::write(parent.join("uv.toml"), "offline = true\n").unwrap();

    let output = doctor(&project, &bin, &uv);
    let report = text(&output);
    assert_eq!(output.status.code(), Some(0), "{output:?}");
    assert!(report.contains("group \"quality\" is not checked"), "{report}");
    assert!(!report.contains("group \"dev\" is not checked"), "{report}");

    fs::remove_file(parent.join("uv.toml")).unwrap();
    fs::write(
        parent.join("pyproject.toml"),
        "[tool.uv]\ndefault-groups = ['quality']\n",
    )
    .unwrap();
    let inherited = doctor(&project, &bin, &uv);
    let report = text(&inherited);
    assert_eq!(inherited.status.code(), Some(0), "{inherited:?}");
    assert!(report.contains("group \"quality\" is not checked"), "{report}");
    assert!(!report.contains("group \"dev\" is not checked"), "{report}");

    fs::write(project.join("pyproject.toml"), "[project]\nname = 'sample'\nversion = '0.1.0'\n[dependency-groups]\ndev = []\nquality = []\n[tool.uv]\ndefault-groups = ['dev']\n").unwrap();
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
    let metadata = project.join("pyproject.toml");
    fs::write(
        &metadata,
        "[project]\nname = 'sample'\nversion = '0.1.0'\n[dependency-groups]\nquality = []\n[tool.uv]\ndefault-groups = ['quality']\ndev-dependencies = [42]\n",
    )
    .unwrap();

    let output = doctor(&project, &bin, &uv);
    let report = text(&output);
    assert_eq!(output.status.code(), Some(1), "{output:?}");
    assert!(
        report.contains("[tool.uv].dev-dependencies")
            && report.contains("must contain only requirement strings"),
        "{report}"
    );
    assert!(report.contains("pyproject.toml"), "{report}");

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
        fs::write(
            &metadata,
            format!("[project]\nname = 'sample'\nversion = '0.1.0'\n{groups}"),
        )
        .unwrap();
        let output = doctor(&project, &bin, &uv);
        let report = text(&output);
        assert_eq!(output.status.code(), Some(1), "{output:?}");
        assert!(report.contains(expected), "{report}");
        assert!(!report.contains("OK: dependencies:"), "{report}");
    }
    fs::write(
        &metadata,
        "[project]\nname = 'sample'\nversion = '0.1.0'\n[tool.uv]\ndefault-groups = ['missing']\n",
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
    assert_eq!(excluded_include.status.code(), Some(1), "{excluded_include:?}");
    assert!(
        report.contains("missing-doctor-quality-package is missing"),
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

    fs::write(&metadata, "[project]\nname = 'sample'\nversion = '1.0.0'\n[dependency-groups]\ndev = ['missing-doctor-dev-package==1.0']\nquality = ['missing-doctor-quality-package==1.0']\n[tool.uv]\ndefault-groups = ['quality']\n").unwrap();
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
        report.contains("missing-doctor-quality-package is missing from .venv"),
        "{report}"
    );
    assert!(
        !report.contains("missing-doctor-dev-package is missing"),
        "{report}"
    );

    fs::write(&metadata, "[project]\nname = 'sample'\nversion = '1.0.0'\n[dependency-groups]\ndev = []\nquality = []\n[tool.uv]\ndefault-groups = ['quality']\ndev-dependencies = ['broken =>1']\n").unwrap();
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
            fs::canonicalize(&metadata).unwrap().display()
        )),
        "{report}"
    );

    fs::write(&metadata, "[project]\nname = 'sample'\nversion = '1.0.0'\n[dependency-groups]\ndev = []\nquality = []\n[tool.uv]\ndefault-groups = ['quality']\ndev-dependencies = ['missing-doctor-legacy-package==1.0']\n").unwrap();
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

    fs::write(&metadata, "[project]\nname = 'sample'\nversion = '1.0.0'\n[dependency-groups]\ndev = []\nquality = []\n[tool.uv]\ndefault-groups = ['dev']\ndev-dependencies = ['missing-doctor-legacy-package==1.0']\n").unwrap();
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

    fs::write(&metadata, "[project]\nname = 'sample'\nversion = '1.0.0'\n[dependency-groups]\nqa = [{include-group = 'dev'}]\ndev = []\n[tool.uv]\ndefault-groups = ['qa']\ndev-dependencies = ['missing-doctor-legacy-package==1.0']\n").unwrap();
    let included_legacy =
        doctor_command(&project, Path::new(&path), &uv).output().unwrap();
    assert_eq!(included_legacy.status.code(), Some(0), "{included_legacy:?}");
    assert!(
        !text(&included_legacy)
            .contains("missing-doctor-legacy-package is missing from .venv")
    );
    for (name, value) in [("UV_NO_DEV", "1"), ("UV_NO_GROUP", "dev")] {
        let included_with_exclusion =
            doctor_command(&project, Path::new(&path), &uv)
                .env(name, value)
                .output()
                .unwrap();
        assert_eq!(
            included_with_exclusion.status.code(),
            Some(0),
            "{included_with_exclusion:?}"
        );
        assert!(
            !text(&included_with_exclusion).contains(
                "missing-doctor-legacy-package is missing from .venv"
            )
        );
    }

    fs::write(&metadata, "[project]\nname = 'sample'\nversion = '1.0.0'\n[tool.uv]\ndefault-groups = ['dev']\ndev-dependencies = ['missing-doctor-legacy-package==1.0']\n")
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

#[test]
fn project_environment_override_is_used_for_layout_and_dependency_probes() {
    let sandbox = tempdir().unwrap();
    let project = sandbox.path().join("project");
    fs::create_dir(&project).unwrap();
    fs::write(project.join("pyproject.toml"), "[project]\nname = 'sample'\nversion = '1.0.0'\nrequires-python = '>=3.10'\ndependencies = []\n").unwrap();
    for override_path in
        [PathBuf::from("custom-env"), sandbox.path().join("absolute-env")]
    {
        let environment = if override_path.is_absolute() {
            override_path.clone()
        } else {
            project.join(&override_path)
        };
        let sync = isolated_command(&project)
            .arg("sync")
            .arg("--offline")
            .arg("--project")
            .arg(&project)
            .env("UV_PROJECT_ENVIRONMENT", &override_path)
            .env("UV_CACHE_DIR", sandbox.path().join("cache"))
            .output()
            .unwrap();
        assert!(sync.status.success(), "{sync:?}");
        assert!(environment.is_dir());
        let scripts =
            environment.join(if cfg!(windows) { "Scripts" } else { "bin" });
        install(&scripts, "ruff");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let stale = project.join(".venv/bin/python");
            fs::create_dir_all(stale.parent().unwrap()).unwrap();
            fs::write(&stale, "#!/bin/sh\nexit 44\n").unwrap();
            fs::set_permissions(&stale, fs::Permissions::from_mode(0o755))
                .unwrap();
        }

        fs::write(project.join("pyproject.toml"), "[project]\nname = 'sample'\nversion = '1.0.0'\nrequires-python = '>=3.10'\ndependencies = ['missing-doctor-fixture-package==1.0']\n").unwrap();
        let output = isolated_command(&project)
            .arg("doctor")
            .arg("--project")
            .arg(&project)
            .env("UV_PROJECT_ENVIRONMENT", &override_path)
            .output()
            .unwrap();
        let report = text(&output);
        assert_eq!(output.status.code(), Some(1), "{output:?}");
        assert!(report.contains("OK: Python:"), "{report}");
        let ruff_line =
            report.lines().find(|line| line.starts_with("OK: ruff:")).unwrap();
        assert!(
            ruff_line
                .contains(environment.file_name().unwrap().to_str().unwrap()),
            "{report}"
        );
        assert!(!report.contains("ERROR: layout:"), "{report}");
        assert!(
            report.lines().any(|line| line.contains("is missing from")
                && line.contains(
                    environment.file_name().unwrap().to_str().unwrap()
                )),
            "{report}"
        );
        assert!(!report.contains("project interpreter"), "{report}");

        fs::write(project.join("pyproject.toml"), "[project]\nname = 'sample'\nversion = '1.0.0'\nrequires-python = '>=3.10'\ndependencies = []\n").unwrap();
    }
}

#[cfg(unix)]
#[test]
fn system_prefix_python3_is_used_for_layout_and_dependencies() {
    use std::os::unix::fs::symlink;
    let sandbox = tempdir().unwrap();
    let project = sandbox.path().join("project");
    let prefix = sandbox.path().join("system-prefix");
    let bin = sandbox.path().join("bin");
    fs::create_dir_all(prefix.join("bin")).unwrap();
    fs::create_dir(&project).unwrap();
    let uv = install(&bin, "uv");
    symlink(which::which("python3").unwrap(), prefix.join("bin/python3"))
        .unwrap();
    fs::write(project.join("pyproject.toml"), "[project]\nname = 'sample'\nversion = '0.1.0'\ndependencies = ['missing-doctor-fixture-package==1.0']\n").unwrap();
    let output = doctor_command(&project, &bin, &uv)
        .env("UV_PROJECT_ENVIRONMENT", &prefix)
        .output()
        .unwrap();
    let report = text(&output);
    assert_eq!(output.status.code(), Some(1), "{output:?}");
    assert!(
        report.contains("OK: Python:") && report.contains("system-prefix"),
        "{report}"
    );
    assert!(
        !report.contains("ERROR: layout:")
            && !report.contains("project interpreter"),
        "{report}"
    );
    assert!(
        report.contains("missing-doctor-fixture-package is missing from")
            && report.contains("system-prefix"),
        "{report}"
    );
}
