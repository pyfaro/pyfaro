use toml_edit::{DocumentMut, Item, Table, value};

use crate::error::{Error, Result};

pub(super) fn is_workspace(source: &str) -> Result<bool> {
    let document: DocumentMut = source.parse()?;
    Ok(document
        .get("tool")
        .and_then(|tool| tool.get("uv"))
        .and_then(|uv| uv.get("workspace"))
        .is_some())
}

/// Plan the opt-in change while preserving unrelated TOML formatting.
pub(super) fn prepare(original: &str) -> Result<String> {
    let mut document: DocumentMut = original.parse()?;
    if document.get("project").and_then(Item::as_table_like).is_none() {
        return Err(Error::Project(
            "expected a standard [project] table in pyproject.toml".into(),
        ));
    }
    if document.get("tool").is_none() {
        let mut tool = Table::new();
        tool.set_implicit(true);
        document["tool"] = Item::Table(tool);
    }
    let tool = document["tool"]
        .as_table_like_mut()
        .ok_or_else(|| Error::Project("tool must be a TOML table".into()))?;
    if tool.get("pyfaro").is_none() {
        tool.insert("pyfaro", Item::Table(Table::new()));
    }
    let settings =
        tool.get_mut("pyfaro").and_then(Item::as_table_like_mut).ok_or_else(
            || Error::Project("[tool.pyfaro] must be a table".into()),
        )?;
    for (key, _) in settings.iter() {
        if key != "schema-version" {
            return Err(Error::Project(format!(
                "unknown [tool.pyfaro] setting: {key}"
            )));
        }
    }
    if let Some(version) = settings.get("schema-version")
        && version.as_integer() != Some(1)
    {
        return Err(Error::Project(
            "tool.pyfaro.schema-version must be the integer 1".into(),
        ));
    }
    if settings.get("schema-version").is_none() {
        settings.insert("schema-version", value(1));
    }
    Ok(document.to_string())
}

#[cfg(test)]
mod tests {
    use super::prepare;

    #[test]
    fn rejects_unknown_or_invalid_pyfaro_settings() {
        for settings in [
            "schema-version = 2",
            "schema-version = '1'",
            "schema-version = true",
            "unexpected-setting = true",
        ] {
            let source = format!(
                "[project]\nname = 'example'\n\n[tool.pyfaro]\n{settings}\n"
            );
            assert!(prepare(&source).is_err(), "accepted {settings}");
        }
    }

    #[test]
    fn complete_configuration_roundtrips_comments_inline_and_dotted_tables() {
        for source in [
            "# comment\n[project]\nname = 'example'\n\n[tool.pyfaro]\nschema-version=1 # keep\n",
            "tool = { pyfaro = { schema-version = 1 }, ruff = { line-length = 100 } }\n[project]\nname = 'example'\n",
            "tool.pyfaro.schema-version = 1\n[project]\nname = 'example'\n",
        ] {
            assert_eq!(prepare(source).unwrap(), source);
        }
    }

    #[test]
    fn rejects_malformed_or_nonstandard_project_configuration() {
        for source in [
            "[project",
            "[tool.poetry]\nname = 'legacy'\n",
            "project = true\n",
            "tool = false\n[project]\nname = 'example'\n",
            "[project]\nname = 'example'\n[tool]\npyfaro = true\n",
        ] {
            assert!(prepare(source).is_err(), "accepted {source}");
        }
    }
}
