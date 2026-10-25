use std::fs;
use std::path::Path;

use pep440_rs::{Version, VersionSpecifiers};

use crate::error::Result;

use super::{Report, Severity};

pub(super) fn inspect_python_requirement(
    requirement: Option<&str>,
    reported_version: &str,
    report: &mut Report,
) -> Result<()> {
    let Some(requirement) = requirement else {
        return report.line(
            Severity::Error,
            "configuration: [project].requires-python must be a version specifier string",
        );
    };
    let Ok(specifiers) = requirement.parse::<VersionSpecifiers>() else {
        return report.line(
            Severity::Error,
            format!("configuration: invalid requires-python {requirement:?}; correct pyproject.toml"),
        );
    };
    let version = reported_version
        .strip_prefix("Python ")
        .and_then(|rest| rest.split_whitespace().next())
        .and_then(|number| number.parse::<Version>().ok());
    if let Some(version) = version
        && !specifiers.contains(&version)
    {
        return report.line(
            Severity::Error,
            format!("Python: {reported_version} does not satisfy requires-python {requirement:?}; select a compatible interpreter"),
        );
    }
    Ok(())
}

pub(super) fn inspect_python_pin(
    root: &Path,
    reported_version: Option<&str>,
    report: &mut Report,
) -> Result<()> {
    let path = root.join(".python-version");
    let contents = match fs::metadata(&path) {
        Ok(metadata) if metadata.is_file() => fs::read_to_string(&path),
        Ok(_) => {
            return report.line(Severity::Error, format!("Python: {} must be a readable file, not a directory or special file; repair the version pin", path.display()));
        }
        Err(error) => Err(error),
    };
    let request =
        match contents {
            Ok(request) => request,
            Err(error)
                if error.kind() == std::io::ErrorKind::NotFound
                    && !path.is_symlink() =>
            {
                return Ok(());
            }
            Err(error) => return report.line(
                Severity::Error,
                format!(
                    "Python: cannot read {}: {error}; repair the version pin",
                    path.display()
                ),
            ),
        };
    let request = request.trim();
    let numeric = request.strip_prefix("cpython@").unwrap_or(request);
    let components: Option<Vec<u64>> = numeric
        .split('.')
        .map(|part| {
            if part.is_empty()
                || !part.bytes().all(|byte| byte.is_ascii_digit())
            {
                None
            } else {
                part.parse().ok()
            }
        })
        .collect();
    let Some(components) =
        components.filter(|parts| (2..=3).contains(&parts.len()))
    else {
        return report.line(Severity::Warn, format!("Python: cannot compare .python-version request {request:?} with the selected interpreter; verify the pin"));
    };
    let Some(reported_version) = reported_version else {
        return Ok(());
    };
    let selected = reported_version
        .strip_prefix("Python ")
        .and_then(|rest| rest.split_whitespace().next())
        .and_then(|number| number.parse::<Version>().ok());
    if selected
        .is_some_and(|version| !version.release().starts_with(&components))
    {
        report.line(Severity::Error, format!("Python: {reported_version} does not match .python-version {request:?}; select a matching interpreter or update the pin"))?;
    }
    Ok(())
}
