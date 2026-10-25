use std::ffi::OsString;
use std::path::Path;
use std::sync::atomic::AtomicBool;

use pep440_rs::Version;

use crate::cli::Tool;
use crate::error::{Error, Result};
use crate::tools;

use super::{Report, Severity};

/// The tool name is already printed as the line prefix, so drop it from the
/// error text.
pub(super) fn failure_detail(error: Error) -> String {
    match error {
        Error::Tool { reason, .. } => reason,
        other => other.to_string(),
    }
}

pub(super) fn inspect_tool(
    tool: Tool,
    root: Option<&Path>,
    uv: Option<&OsString>,
    cancelled: &AtomicBool,
    report: &mut Report,
    required: bool,
) -> Result<Option<String>> {
    let name = tools::name(tool);
    match tools::inspect(tool, root, uv, cancelled) {
        Ok((path, version)) => {
            report.line(
                Severity::Ok,
                format!("{name}: {} ({version})", path.display()),
            )?;
            Ok(Some(version))
        }
        Err(Error::Cancelled) => Err(Error::Cancelled),
        Err(error) => {
            let detail = failure_detail(error);
            report.line(
                if required { Severity::Error } else { Severity::Warn },
                format!("{name}: {detail}"),
            )?;
            Ok(None)
        }
    }
}

/// Reports pytest 9-only configuration when the found pytest is older.
pub(super) fn check_pytest_9_configuration(
    version: &str,
    report: &mut Report,
) -> Result<()> {
    let number = version.split_whitespace().nth(1).unwrap_or_default();
    let old = number
        .parse::<Version>()
        .ok()
        .and_then(|found| found.release().first().copied())
        .is_some_and(|major| major < 9);
    if old {
        report.line(Severity::Error, format!("configuration: pytest {number} cannot use pytest 9 configuration; upgrade to pytest >=9 or use INI configuration"))?;
    }
    Ok(())
}
