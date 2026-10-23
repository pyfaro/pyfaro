use std::fs;
use std::path::{Path, PathBuf};

use toml_edit::{
    DocumentMut, InlineTable, Item, Table, TableLike, Value, value,
};

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

    /// The validated `[tool.pyfaro.dev]` table, if the project has one.
    pub(crate) fn dev(&self) -> Result<Option<DevConfig>> {
        let document = Self::load_document(self.root())?;
        let settings = pyfaro_table(&document);
        settings
            .and_then(|settings| settings.get("dev"))
            .map(parse_dev)
            .transpose()
            .map_err(|error| {
                Error::config(&self.root().join("pyproject.toml"), error)
            })
    }
}

/// A preset supplies the runner and defaults for `[tool.pyfaro.dev]`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum DevPreset {
    FastApi,
}

/// Typed `[tool.pyfaro.dev]` settings, validated before anything is launched.
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct DevConfig {
    pub preset: Option<DevPreset>,
    pub entry_point: String,
    pub args: Vec<String>,
    pub reload: Option<bool>,
}

fn pyfaro_table(document: &DocumentMut) -> Option<&dyn TableLike> {
    document.get("tool")?.as_table_like()?.get("pyfaro")?.as_table_like()
}

fn parse_dev(item: &Item) -> Result<DevConfig> {
    let table = item.as_table_like().ok_or_else(|| {
        Error::Project("tool.pyfaro.dev must be a table".into())
    })?;
    for (key, _) in table.iter() {
        if !matches!(key, "preset" | "entry-point" | "args" | "reload") {
            return Err(Error::Project(format!(
                "unknown [tool.pyfaro.dev] setting: {key}"
            )));
        }
    }
    let preset = match table.get("preset") {
        None => None,
        Some(item) if item.as_str() == Some("fastapi") => {
            Some(DevPreset::FastApi)
        }
        Some(_) => {
            return Err(Error::Project(
                "tool.pyfaro.dev.preset must be the string \"fastapi\"".into(),
            ));
        }
    };
    let entry_point = table
        .get("entry-point")
        .ok_or_else(|| {
            Error::Project("tool.pyfaro.dev.entry-point is required".into())
        })?
        .as_str()
        .ok_or_else(|| {
            Error::Project(
                "tool.pyfaro.dev.entry-point must be a string".into(),
            )
        })?;
    validate_entry_point(entry_point, preset)?;
    let args = match table.get("args") {
        None => Vec::new(),
        Some(item) => item
            .as_array()
            .and_then(|array| {
                array
                    .iter()
                    .map(|arg| arg.as_str().map(str::to_owned))
                    .collect()
            })
            .ok_or_else(|| {
                Error::Project(
                    "tool.pyfaro.dev.args must be an array of strings".into(),
                )
            })?,
    };
    let reload = match table.get("reload") {
        None => None,
        Some(item) => Some(item.as_bool().ok_or_else(|| {
            Error::Project("tool.pyfaro.dev.reload must be a boolean".into())
        })?),
    };
    if reload.is_some() && preset.is_none() {
        return Err(Error::Project(
            "tool.pyfaro.dev.reload requires preset = \"fastapi\"; Pyfaro does not watch files itself"
                .into(),
        ));
    }
    Ok(DevConfig { preset, entry_point: entry_point.to_owned(), args, reload })
}

fn validate_entry_point(
    entry_point: &str,
    preset: Option<DevPreset>,
) -> Result<()> {
    let valid = match preset {
        Some(DevPreset::FastApi) => {
            entry_point.split_once(':').is_some_and(|(module, attribute)| {
                is_dotted_name(module) && is_identifier(attribute)
            })
        }
        None => is_dotted_name(entry_point),
    };
    if valid {
        return Ok(());
    }
    Err(Error::Project(match preset {
        Some(DevPreset::FastApi) => format!(
            "tool.pyfaro.dev.entry-point must be an ASGI import string like \"package.module:app\" with the fastapi preset, found {entry_point:?}"
        ),
        None => format!(
            "tool.pyfaro.dev.entry-point must be a module name like \"package.module\" (run as python -m) without a preset, found {entry_point:?}"
        ),
    }))
}

fn is_dotted_name(name: &str) -> bool {
    name.split('.').all(is_identifier)
}

fn is_identifier(name: &str) -> bool {
    let mut characters = name.chars();
    characters.next().is_some_and(|first| first == '_' || first.is_alphabetic())
        && characters.all(|next| next == '_' || next.is_alphanumeric())
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
        if !matches!(key, "schema-version" | "dev" | "tasks") {
            return Err(Error::Project(format!(
                "unknown [tool.pyfaro] setting: {key}"
            )));
        }
    }
    if let Some(dev) = settings.get("dev") {
        parse_dev(dev)?;
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

    fn dev_error(table: &str) -> String {
        let source = format!(
            "[project]\nname = 'example'\n\n[tool.pyfaro.dev]\n{table}\n"
        );
        prepare(&source).unwrap_err().to_string()
    }

    fn dev_table(table: &str) -> super::DevConfig {
        let directory = tempfile::tempdir().unwrap();
        std::fs::write(
            directory.path().join("pyproject.toml"),
            format!(
                "[project]\nname = 'example'\n\n[tool.pyfaro.dev]\n{table}\n"
            ),
        )
        .unwrap();
        Config::load(directory.path()).unwrap().dev().unwrap().unwrap()
    }

    #[test]
    fn dev_settings_are_typed_and_optional() {
        let directory = tempfile::tempdir().unwrap();
        std::fs::write(
            directory.path().join("pyproject.toml"),
            "[project]\nname = 'example'\n",
        )
        .unwrap();
        let config = Config::load(directory.path()).unwrap();
        assert_eq!(config.dev().unwrap(), None);

        let full = dev_table(
            "preset = 'fastapi'\nentry-point = 'demo.main:app'\nargs = ['--port', '8001']\nreload = false",
        );
        assert_eq!(full.preset, Some(super::DevPreset::FastApi));
        assert_eq!(full.entry_point, "demo.main:app");
        assert_eq!(full.args, ["--port", "8001"]);
        assert_eq!(full.reload, Some(false));

        let module = dev_table("entry-point = 'demo.worker'\nargs = []");
        assert_eq!(module.preset, None);
        assert_eq!(module.reload, None);
        assert!(module.args.is_empty());
    }

    #[test]
    fn dev_validation_names_the_offending_key() {
        for (table, expected) in [
            ("", "tool.pyfaro.dev.entry-point is required"),
            (
                "entry-point = 'a'\nbogus = 1",
                "unknown [tool.pyfaro.dev] setting: bogus",
            ),
            (
                "preset = 'flask'\nentry-point = 'a:app'",
                "tool.pyfaro.dev.preset",
            ),
            ("preset = 1\nentry-point = 'a:app'", "tool.pyfaro.dev.preset"),
            ("entry-point = 3", "tool.pyfaro.dev.entry-point must be a string"),
            ("entry-point = 'a:app'", "without a preset"),
            ("entry-point = ''", "tool.pyfaro.dev.entry-point"),
            ("entry-point = 'a..b'", "tool.pyfaro.dev.entry-point"),
            ("entry-point = '1a'", "tool.pyfaro.dev.entry-point"),
            ("preset = 'fastapi'\nentry-point = 'a.b'", "ASGI import string"),
            ("preset = 'fastapi'\nentry-point = 'a:b:c'", "ASGI import string"),
            ("preset = 'fastapi'\nentry-point = 'a:b.c'", "ASGI import string"),
            ("preset = 'fastapi'\nentry-point = ':app'", "ASGI import string"),
            (
                "entry-point = 'a'\nargs = 'x'",
                "tool.pyfaro.dev.args must be an array of strings",
            ),
            (
                "entry-point = 'a'\nargs = ['x', 1]",
                "tool.pyfaro.dev.args must be an array of strings",
            ),
            (
                "preset = 'fastapi'\nentry-point = 'a:b'\nreload = 'yes'",
                "tool.pyfaro.dev.reload must be a boolean",
            ),
            (
                "entry-point = 'a'\nreload = true",
                "reload requires preset = \"fastapi\"; Pyfaro does not watch files itself",
            ),
        ] {
            let message = dev_error(table);
            assert!(message.contains(expected), "{table}: {message}");
        }
    }

    #[test]
    fn dev_must_be_a_table() {
        let source = "[project]\nname = 'example'\n[tool.pyfaro]\ndev = 1\n";
        assert!(prepare(source).is_err());
    }
}
