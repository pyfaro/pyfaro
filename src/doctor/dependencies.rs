use std::collections::HashMap;
use std::env;
use std::fs;
use std::path::Path;
use std::process::{Command, Stdio};
use std::sync::atomic::AtomicBool;

use toml_edit::{DocumentMut, Table};

use crate::error::{Error, Result};
use crate::{process, tools};

use super::uv_config::{UvConfiguration, uv_configuration, uv_env_enabled};
use super::{Report, Severity};

fn normalize_group_name(name: &str) -> String {
    let mut normalized = String::with_capacity(name.len());
    let mut separator = false;
    for character in name.chars() {
        if matches!(character, '-' | '_' | '.') {
            if !separator {
                normalized.push('-');
            }
            separator = true;
        } else {
            normalized.push(character.to_ascii_lowercase());
            separator = false;
        }
    }
    normalized
}

pub(super) fn valid_group_name(name: &str) -> bool {
    let bytes = name.as_bytes();
    bytes.first().is_some_and(u8::is_ascii_alphanumeric)
        && bytes.last().is_some_and(u8::is_ascii_alphanumeric)
        && bytes.iter().all(|byte| {
            byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.')
        })
}

struct GroupCollector<'a> {
    groups: &'a Table,
    names: &'a HashMap<String, String>,
    source: &'a Path,
    checked: Vec<String>,
    selected: Vec<String>,
    active: Vec<String>,
}

impl GroupCollector<'_> {
    fn collect(
        &mut self,
        name: &str,
        requirements: &mut Vec<Requirement>,
        unselected_requirements: &mut Vec<Requirement>,
        selected_group: bool,
        report: &mut Report,
    ) -> Result<bool> {
        if self.active.iter().any(|entry| entry == name) {
            report.line(
                Severity::Error,
                format!(
                    "dependencies: dependency group cycle includes {name:?}"
                ),
            )?;
            return Ok(false);
        }
        if self.checked.iter().any(|entry| entry == name)
            && (!selected_group
                || self.selected.iter().any(|entry| entry == name))
        {
            return Ok(true);
        }
        let Some(original) = self.names.get(name).cloned() else {
            report.line(
                Severity::Error,
                format!("dependencies: group {name:?} is missing; define it in [dependency-groups]"),
            )?;
            return Ok(false);
        };
        let groups = self.groups;
        let Some(group) =
            groups.get(&original).and_then(|group| group.as_array())
        else {
            report.line(
                Severity::Error,
                format!("dependencies: group {original:?} must be an array"),
            )?;
            return Ok(false);
        };
        self.active.push(name.to_owned());
        let mut valid = true;
        for entry in group.iter() {
            if let Some(requirement) = entry.as_str() {
                let target = if selected_group {
                    &mut *requirements
                } else {
                    &mut *unselected_requirements
                };
                target.push(Requirement {
                    text: requirement.to_owned(),
                    source: self.source.to_owned(),
                });
            } else {
                let included = entry
                    .as_inline_table()
                    .filter(|table| table.len() == 1)
                    .and_then(|table| table.get("include-group"))
                    .and_then(|value| value.as_str());
                if let Some(included) = included {
                    let included = normalize_group_name(included);
                    valid &= self.collect(
                        &included,
                        requirements,
                        unselected_requirements,
                        selected_group,
                        report,
                    )?;
                } else {
                    report.line(
                        Severity::Error,
                        format!("dependencies: group {original:?} contains an invalid entry; use a requirement string or an include-group table with no extra keys"),
                    )?;
                    valid = false;
                }
            }
        }
        self.active.pop();
        self.checked.push(name.to_owned());
        if selected_group {
            self.selected.push(name.to_owned());
        }
        Ok(valid)
    }
}

pub(super) struct DeclaredDependencies {
    requirements: Vec<Requirement>,
    selected_count: usize,
}

struct Requirement {
    text: String,
    source: std::path::PathBuf,
}

pub(super) fn declared_dependencies(
    root: &Path,
    document: &DocumentMut,
    uv_binary: Option<&Path>,
    cancelled: &AtomicBool,
    report: &mut Report,
) -> Result<Option<DeclaredDependencies>> {
    let mut requirements = Vec::new();
    let mut invalid = false;
    let metadata = root.join("pyproject.toml");
    if let Some(dependencies) =
        document.get("project").and_then(|project| project.get("dependencies"))
    {
        if let Some(dependencies) = dependencies.as_array() {
            for dependency in dependencies.iter() {
                if let Some(requirement) = dependency.as_str() {
                    requirements.push(Requirement {
                        text: requirement.to_owned(),
                        source: metadata.clone(),
                    });
                } else {
                    report.line(Severity::Error, "dependencies: each [project].dependencies entry must be a string")?;
                    invalid = true;
                }
            }
        } else {
            report.line(Severity::Error, "dependencies: [project].dependencies must be an array of requirement strings")?;
            invalid = true;
        }
    }
    if document
        .get("project")
        .and_then(|project| project.get("dynamic"))
        .and_then(|dynamic| dynamic.as_array())
        .is_some_and(|dynamic| {
            dynamic.iter().any(|entry| entry.as_str() == Some("dependencies"))
        })
    {
        report.line(Severity::Error, "dependencies: dynamic project dependencies cannot be checked from pyproject.toml; inspect the build backend output and sync the environment")?;
        invalid = true;
    }
    if uv_configuration(root, document, uv_binary, cancelled, report)?.is_none()
    {
        return Ok(None);
    }
    let project_settings = document
        .get("tool")
        .and_then(|tool| tool.get("uv"))
        .map(|_| UvConfiguration {
            document: document.clone(),
            path: metadata.clone(),
            standalone: false,
        });
    let default_setting = project_settings.as_ref().and_then(|config| {
        config.setting("default-groups").map(|value| (config, value))
    });
    let explicit_defaults = default_setting.is_some();
    let has_legacy_dev = project_settings
        .as_ref()
        .is_some_and(|config| config.setting("dev-dependencies").is_some());
    let selected_groups = match default_setting {
        None => Some(vec!["dev".to_owned()]),
        Some((_, value)) if value.as_str() == Some("all") => None,
        Some((config, value)) => match value.as_array() {
            Some(values) => {
                let mut names = Vec::new();
                for entry in values.iter() {
                    if let Some(name) = entry.as_str() {
                        names.push(normalize_group_name(name));
                    } else {
                        report.line(
                            Severity::Error,
                            format!(
                                "dependencies: {} must contain only group names",
                                config.label("default-groups")
                            ),
                        )?;
                        invalid = true;
                    }
                }
                Some(names)
            }
            None => {
                report.line(
                    Severity::Error,
                    format!(
                        "dependencies: {} must be a list or 'all'",
                        config.label("default-groups")
                    ),
                )?;
                invalid = true;
                Some(Vec::new())
            }
        },
    };
    let disabled_groups: Vec<String> = env::var("UV_NO_GROUP")
        .unwrap_or_default()
        .split_whitespace()
        .map(normalize_group_name)
        .chain(uv_env_enabled("UV_NO_DEV").then(|| "dev".to_owned()))
        .collect();
    let no_default_groups = uv_env_enabled("UV_NO_DEFAULT_GROUPS");
    let mut unselected_requirements = Vec::new();
    if let Some(groups) = document.get("dependency-groups") {
        let groups = groups.as_table().cloned().or_else(|| {
            groups.as_inline_table().map(|inline| inline.clone().into_table())
        });
        if let Some(groups) = groups {
            let mut group_names = HashMap::new();
            for (name, _) in groups.iter() {
                if !valid_group_name(name) {
                    report.line(Severity::Error, format!("dependencies: group name {name:?} is invalid; use an ASCII name starting and ending with a letter or digit"))?;
                    invalid = true;
                }
                let normalized = normalize_group_name(name);
                if let Some(previous) =
                    group_names.insert(normalized.clone(), name.to_owned())
                {
                    report.line(Severity::Error, format!("dependencies: duplicate dependency group names {previous:?} and {name:?} normalize to {normalized:?}; rename one group"))?;
                    invalid = true;
                }
            }
            let mut collector = GroupCollector {
                groups: &groups,
                names: &group_names,
                source: &metadata,
                checked: Vec::new(),
                selected: Vec::new(),
                active: Vec::new(),
            };
            let configured_names: Vec<String> = match &selected_groups {
                Some(selected) => selected.clone(),
                None => groups
                    .iter()
                    .map(|(name, _)| normalize_group_name(name))
                    .collect(),
            };
            for name in &configured_names {
                let implicit_dev =
                    name == "dev" && (has_legacy_dev || !explicit_defaults);
                if !group_names.contains_key(name) && !implicit_dev {
                    report.line(Severity::Error, format!("dependencies: group {name:?} is missing; define it in [dependency-groups]"))?;
                    invalid = true;
                }
            }
            for name in configured_names {
                if no_default_groups
                    || disabled_groups.iter().any(|disabled| disabled == &name)
                    || !group_names.contains_key(&name)
                {
                    continue;
                }
                invalid |= !collector.collect(
                    &name,
                    &mut requirements,
                    &mut unselected_requirements,
                    true,
                    report,
                )?;
            }
            for (name, _) in groups.iter() {
                let normalized = normalize_group_name(name);
                if !collector.checked.iter().any(|entry| entry == &normalized) {
                    invalid |= !collector.collect(
                        &normalized,
                        &mut requirements,
                        &mut unselected_requirements,
                        false,
                        report,
                    )?;
                }
                if !collector.selected.iter().any(|entry| entry == &normalized)
                {
                    report.line(Severity::Warn, format!("dependencies: group {name:?} is not checked for installed distributions; sync it explicitly if needed"))?;
                }
            }
        } else {
            report.line(
                Severity::Error,
                "dependencies: [dependency-groups] must be a table",
            )?;
            invalid = true;
        }
    } else if explicit_defaults && let Some(selected) = &selected_groups {
        for name in selected {
            if name == "dev" && has_legacy_dev {
                continue;
            }
            report.line(Severity::Error, format!("dependencies: group {name:?} is missing; define it in [dependency-groups]"))?;
            invalid = true;
        }
    }
    for config in project_settings.iter() {
        let Some(legacy) = config.setting("dev-dependencies") else {
            continue;
        };
        let label = config.label("dev-dependencies");
        let selected = selected_groups
            .as_ref()
            .is_none_or(|selected| selected.iter().any(|name| name == "dev"))
            && !no_default_groups
            && !disabled_groups.iter().any(|name| name == "dev");
        if let Some(legacy) = legacy.as_array() {
            for entry in legacy.iter() {
                if let Some(requirement) = entry.as_str() {
                    let target = if selected {
                        &mut requirements
                    } else {
                        &mut unselected_requirements
                    };
                    target.push(Requirement {
                        text: requirement.to_owned(),
                        source: config.path.clone(),
                    });
                } else {
                    report.line(Severity::Error, format!("dependencies: {label} must contain only requirement strings; correct {}", config.path.display()))?;
                    invalid = true;
                }
            }
        } else {
            report.line(
                Severity::Error,
                format!(
                    "dependencies: {label} must be an array; correct {}",
                    config.path.display()
                ),
            )?;
            invalid = true;
        }
    }
    let selected_count = requirements.len();
    requirements.extend(unselected_requirements);
    Ok((!invalid)
        .then_some(DeclaredDependencies { requirements, selected_count }))
}

const REQUIREMENT_PROBE: &str = r#"
import pathlib
import sys

try:
    from packaging.requirements import Requirement, InvalidRequirement
    from packaging.utils import canonicalize_name
except ImportError:
    try:
        from pip._vendor.packaging.requirements import Requirement, InvalidRequirement
        from pip._vendor.packaging.utils import canonicalize_name
    except ImportError:
        import ensurepip
        bundled = pathlib.Path(ensurepip.__file__).parent / '_bundled'
        wheels = sorted(bundled.glob('pip-*.whl'))
        if not wheels:
            raise RuntimeError('PyPA packaging is unavailable; install packaging or pip in the selected Python environment')
        sys.path.insert(0, str(wheels[-1]))
        from pip._vendor.packaging.requirements import Requirement, InvalidRequirement
        from pip._vendor.packaging.utils import canonicalize_name

for index, source in enumerate(sys.argv[1:]):
    try:
        requirement = Requirement(source)
        if requirement.marker is None or requirement.marker.evaluate({'extra': ''}):
            print(f'A\t{index}\t{canonicalize_name(requirement.name)}')
    except InvalidRequirement:
        print(f'E\t{index}')
"#;

fn active_dependencies(
    python: &Path,
    requirements: &[Requirement],
    cancelled: &AtomicBool,
    report: &mut Report,
) -> Result<Option<Vec<(usize, String)>>> {
    let output = tempfile::NamedTempFile::new().map_err(|error| {
        Error::io(
            "create requirement output",
            Path::new("temporary file"),
            error,
        )
    })?;
    let errors = tempfile::NamedTempFile::new().map_err(|error| {
        Error::io(
            "create requirement error output",
            Path::new("temporary file"),
            error,
        )
    })?;
    let scratch = tempfile::tempdir().map_err(|error| {
        Error::io(
            "create requirement scratch directory",
            Path::new("temporary directory"),
            error,
        )
    })?;
    let mut command = Command::new(python);
    command
        .current_dir(scratch.path())
        .args(["-I", "-B", "-c", REQUIREMENT_PROBE])
        .args(requirements.iter().map(|requirement| &requirement.text))
        .stdin(Stdio::null())
        .stdout(
            output
                .reopen()
                .map_err(|error| Error::io("open", output.path(), error))?,
        )
        .stderr(
            errors
                .reopen()
                .map_err(|error| Error::io("open", errors.path(), error))?,
        );
    let status = match process::run(command, cancelled) {
        Ok(status) => status,
        Err(Error::Cancelled) => return Err(Error::Cancelled),
        Err(error) => {
            report.line(
                Severity::Error,
                format!("dependencies: could not run requirement parser with {}: {error}; repair the selected Python interpreter or run pyfaro sync", python.display()),
            )?;
            return Ok(None);
        }
    };
    if !status.success() {
        report.line(Severity::Error, format!("dependencies: could not parse requirements with {} ({status}): {}; repair the selected Python interpreter or install packaging", python.display(), process::read_diagnostic(errors.path())?))?;
        return Ok(None);
    }
    let output = fs::read_to_string(output.path())
        .map_err(|error| Error::io("read", output.path(), error))?;
    let mut names = Vec::new();
    let mut invalid = false;
    for line in output.lines() {
        let mut fields = line.split('\t');
        let kind = fields.next();
        let index = fields.next().and_then(|value| value.parse::<usize>().ok());
        let Some((index, requirement)) = index.and_then(|index| {
            requirements.get(index).map(|requirement| (index, requirement))
        }) else {
            report.line(
                Severity::Error,
                "dependencies: invalid requirement probe output",
            )?;
            return Ok(None);
        };
        match (kind, fields.next(), fields.next()) {
            (Some("A"), Some(name), None) if !name.is_empty() => {
                names.push((index, name.to_owned()))
            }
            (Some("E"), None, None) => {
                report.line(
                    Severity::Error,
                    format!(
                        "dependencies: invalid requirement {:?}; correct {}",
                        requirement.text,
                        requirement.source.display()
                    ),
                )?;
                invalid = true;
            }
            _ => {
                report.line(
                    Severity::Error,
                    "dependencies: invalid requirement probe output",
                )?;
                return Ok(None);
            }
        }
    }
    if invalid {
        return Ok(None);
    }
    Ok(Some(names))
}

pub(super) fn inspect_dependencies(
    root: &Path,
    python: Option<&Path>,
    dependencies: &DeclaredDependencies,
    cancelled: &AtomicBool,
    report: &mut Report,
) -> Result<()> {
    if dependencies.requirements.is_empty() {
        return report.line(
            Severity::Ok,
            "dependencies: no runtime dependencies to check",
        );
    }
    let Some(python) = python else {
        return Ok(());
    };
    let environment = tools::project_environment(root);
    let environment_name = if environment == root.join(".venv") {
        ".venv".to_owned()
    } else {
        environment.display().to_string()
    };
    let local = tools::project_python(root);
    let parser_python = if local.is_file() { &local } else { python };
    let declared_count = dependencies.selected_count;
    let Some(dependencies) = active_dependencies(
        parser_python,
        &dependencies.requirements,
        cancelled,
        report,
    )?
    else {
        return Ok(());
    };
    let mut dependencies: Vec<String> = dependencies
        .into_iter()
        .filter(|(index, _)| *index < declared_count)
        .map(|(_, name)| name)
        .collect();
    dependencies.sort_unstable();
    dependencies.dedup();
    if dependencies.is_empty() {
        return report.line(
            Severity::Ok,
            "dependencies: no applicable runtime distributions to check",
        );
    }
    if !local.is_file() {
        return report.line(
            Severity::Error,
            format!("dependencies: project interpreter {} is missing; run pyfaro sync", local.display()),
        );
    }
    let output = tempfile::NamedTempFile::new().map_err(|error| {
        Error::io(
            "create dependency output",
            Path::new("temporary file"),
            error,
        )
    })?;
    let errors = tempfile::NamedTempFile::new().map_err(|error| {
        Error::io(
            "create dependency error output",
            Path::new("temporary file"),
            error,
        )
    })?;
    let mut command = Command::new(&local);
    let scratch = tempfile::tempdir().map_err(|error| {
        Error::io(
            "create dependency scratch directory",
            Path::new("temporary directory"),
            error,
        )
    })?;
    command
        .current_dir(scratch.path())
        .args(["-I", "-B", "-c", "import importlib.metadata as m, sys\nfor name in sys.argv[1:]:\n try: m.version(name)\n except m.PackageNotFoundError: print(name)"])
        .args(&dependencies)
        .stdin(Stdio::null())
        .stdout(output.reopen().map_err(|error| Error::io("open", output.path(), error))?)
        .stderr(errors.reopen().map_err(|error| Error::io("open", errors.path(), error))?);
    let status = match process::run(command, cancelled) {
        Ok(status) => status,
        Err(Error::Cancelled) => return Err(Error::Cancelled),
        Err(error) => return report.line(Severity::Error, format!("dependencies: could not inspect {}: {error}; repair the project environment or run pyfaro sync", local.display())),
    };
    if !status.success() {
        return report.line(Severity::Error, format!("dependencies: interpreter check failed ({status}): {}; repair the project environment or run pyfaro sync", process::read_diagnostic(errors.path())?));
    }
    let missing = fs::read_to_string(output.path())
        .map_err(|error| Error::io("read", output.path(), error))?;
    if missing.is_empty() {
        report.line(
            Severity::Ok,
            format!(
                "dependencies: {} runtime distribution(s) installed in {}",
                dependencies.len(),
                environment_name
            ),
        )
    } else {
        for name in missing.lines() {
            report.line(Severity::Error, format!("dependencies: {name} is missing from {environment_name}; run pyfaro sync"))?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::{Report, Requirement, active_dependencies};
    use std::sync::atomic::AtomicBool;

    #[test]
    fn requirement_parser_spawn_failure_is_a_blocking_finding() {
        let sandbox = tempfile::tempdir().unwrap();
        let missing_python = sandbox.path().join("missing-python");
        let mut report = Report { blocking: 0 };
        let result = active_dependencies(
            &missing_python,
            &[Requirement {
                text: "example>=1".to_owned(),
                source: sandbox.path().join("pyproject.toml"),
            }],
            &AtomicBool::new(false),
            &mut report,
        )
        .unwrap();
        assert!(result.is_none());
        assert_eq!(report.blocking, 1);
    }
}
