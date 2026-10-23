use std::fs;
use std::path::{Path, PathBuf};

use toml_edit::{DocumentMut, InlineTable, Item, Table, Value, value};

use crate::error::{Error, Result};
use crate::tasks::{self, Tasks};

pub(crate) enum Config {
    V1 { root: PathBuf, tasks: Tasks },
}

impl Config {
    pub(crate) fn load(root: &Path) -> Result<Self> {
        let (_, tasks) = Self::load_parsed(root)?;
        Ok(Self::V1 { root: root.to_owned(), tasks })
    }

    pub(crate) fn load_document(root: &Path) -> Result<DocumentMut> {
        Self::load_parsed(root).map(|(document, _)| document)
    }

    fn load_parsed(root: &Path) -> Result<(DocumentMut, Tasks)> {
        let metadata = root.join("pyproject.toml");
        let source = fs::read_to_string(&metadata).map_err(|error| {
            Error::io("read project metadata", &metadata, error)
        })?;
        let document: DocumentMut = source.parse().map_err(|source| {
            Error::ConfigToml { path: metadata.clone(), source }
        })?;
        let tasks = validate(&document)
            .map_err(|error| Error::config(&metadata, error))?;
        Ok((document, tasks))
    }

    pub(crate) fn root(&self) -> &Path {
        match self {
            Self::V1 { root, .. } => root,
        }
    }

    pub(crate) fn tasks(&self) -> &Tasks {
        match self {
            Self::V1 { tasks, .. } => tasks,
        }
    }
}

pub(crate) fn is_workspace(source: &str) -> Result<bool> {
    let document: DocumentMut = source.parse()?;
    Ok(document
        .get("tool")
        .and_then(|tool| tool.get("uv"))
        .and_then(|uv| uv.get("workspace"))
        .is_some())
}

pub(crate) fn has_inline_config(source: &str) -> Result<bool> {
    let document: DocumentMut = source.parse()?;
    Ok(document.get("tool").is_some_and(Item::is_inline_table)
        || document
            .get("tool")
            .and_then(|tool| tool.get("pyfaro"))
            .is_some_and(Item::is_inline_table))
}

/// Plan the opt-in change while preserving unrelated TOML formatting.
pub(crate) fn prepare(original: &str) -> Result<String> {
    let mut document: DocumentMut = original.parse()?;
    validate(&document)?;
    if document.get("tool").is_none() {
        let mut tool = Table::new();
        tool.set_implicit(true);
        document["tool"] = Item::Table(tool);
    }
    let inline_tool = document["tool"].is_inline_table();
    let tool = document["tool"]
        .as_table_like_mut()
        .ok_or_else(|| Error::Project("tool must be a TOML table".into()))?;
    if tool.get("pyfaro").is_none() {
        let settings = if inline_tool {
            Item::Value(Value::InlineTable(InlineTable::new()))
        } else {
            Item::Table(Table::new())
        };
        tool.insert("pyfaro", settings);
    }
    let settings =
        tool.get_mut("pyfaro").and_then(Item::as_table_like_mut).ok_or_else(
            || Error::Project("[tool.pyfaro] must be a table".into()),
        )?;
    if settings.get("schema-version").is_none() {
        settings.insert("schema-version", value(1));
    }
    Ok(document.to_string())
}

fn validate(document: &DocumentMut) -> Result<Tasks> {
    if document.get("project").and_then(Item::as_table_like).is_none() {
        return Err(Error::Project(
            "expected a standard [project] table in pyproject.toml".into(),
        ));
    }
    let Some(tool) = document.get("tool") else {
        return Ok(Tasks::default());
    };
    let tool = tool
        .as_table_like()
        .ok_or_else(|| Error::Project("tool must be a TOML table".into()))?;
    let Some(settings) = tool.get("pyfaro") else {
        return Ok(Tasks::default());
    };
    let settings = settings.as_table_like().ok_or_else(|| {
        Error::Project("[tool.pyfaro] must be a table".into())
    })?;
    for (key, _) in settings.iter() {
        if !matches!(key, "schema-version" | "tasks") {
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
    match settings.get("tasks") {
        Some(item) => tasks::parse(item),
        None => Ok(Tasks::default()),
    }
}

#[cfg(test)]
mod tests {
    use super::{Config, prepare};

    #[test]
    fn loading_defaults_to_schema_one_without_writing_metadata() {
        let directory = tempfile::tempdir().unwrap();
        let metadata = directory.path().join("pyproject.toml");
        let source = "[project]\nname = 'example'\n\n[tool.ruff]\nline-length = 100\n\n[tool.ty]\n";
        std::fs::write(&metadata, source).unwrap();
        let config = Config::load(directory.path()).unwrap();
        assert_eq!(config.root(), directory.path());
        assert_eq!(std::fs::read_to_string(metadata).unwrap(), source);
    }

    #[test]
    fn malformed_project_metadata_retains_the_parse_error_source() {
        let directory = tempfile::tempdir().unwrap();
        std::fs::write(directory.path().join("pyproject.toml"), "[project\n")
            .unwrap();
        let error = match Config::load(directory.path()) {
            Ok(_) => panic!("malformed metadata was accepted"),
            Err(error) => error,
        };
        assert!(matches!(error, crate::error::Error::ConfigToml { .. }));
        assert!(std::error::Error::source(&error).is_some());
        assert!(error.to_string().contains("pyproject.toml"));
    }

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
    fn init_adds_settings_to_inline_tool_without_losing_other_tools() {
        let source = "tool = { ruff = { line-length = 100 }, ty = {} }\n[project]\nname = 'example'\n";
        let prepared = prepare(source).unwrap();
        assert!(prepared.contains("ruff = { line-length = 100 }"));
        assert!(prepared.contains("ty = {}"));
        assert!(prepared.contains("pyfaro = { schema-version = 1 }"));
        assert_eq!(prepare(&prepared).unwrap(), prepared);
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
