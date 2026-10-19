use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use crate::cli::{CreateArgs, CreateKind, CreateProjectArgs};
use crate::error::{Error, Result};
use crate::project;

enum TemplateKind {
    Library,
    Cli(String),
}

pub(crate) fn run(args: CreateArgs) -> Result<()> {
    let (project, package, kind) = match args.kind {
        CreateKind::Lib(project) => (project, None, TemplateKind::Library),
        CreateKind::Cli(args) => {
            let command = args
                .command
                .unwrap_or_else(|| args.project.name.replace('.', "-"));
            (args.project, args.package, TemplateKind::Cli(command))
        }
    };
    create(project, package, kind)
}

fn create(
    project: CreateProjectArgs,
    package: Option<String>,
    kind: TemplateKind,
) -> Result<()> {
    validate_distribution(&project.name)?;
    validate_python(&project.python)?;
    let mut default_package = normalized_package(&project.name);
    if default_package.as_bytes()[0].is_ascii_digit() {
        default_package.insert(0, '_');
    }
    let package = package.unwrap_or(default_package);
    validate_package(&package)?;
    if let TemplateKind::Cli(command) = &kind {
        validate_command(command)?;
    }

    let selected = project.path.unwrap_or_else(|| PathBuf::from(&project.name));
    let root = project::target(&selected)?;
    let parent = root.parent().ok_or_else(|| {
        Error::Project(format!("target has no parent: {}", root.display()))
    })?;
    if !parent.is_dir() {
        return Err(Error::Project(format!(
            "target parent must be an existing directory: {}",
            parent.display()
        )));
    }
    let command = match &kind {
        TemplateKind::Library => None,
        TemplateKind::Cli(command) => Some(command.as_str()),
    };
    create_directory(&root, |root| {
        write_project(root, &project.name, &package, &project.python, command)
    })?;
    writeln!(
        io::stdout().lock(),
        "Created {} at {}",
        if matches!(kind, TemplateKind::Cli(_)) { "CLI" } else { "library" },
        root.display()
    )
    .map_err(Error::Output)?;
    Ok(())
}

fn create_directory(
    root: &Path,
    write: impl FnOnce(&Path) -> Result<()>,
) -> Result<()> {
    fs::create_dir(root)
        .map_err(|error| Error::io("create project directory", root, error))?;
    if let Err(operation) = write(root) {
        return match fs::remove_dir_all(root) {
            Ok(()) => Err(operation),
            Err(source) => Err(Error::OperationCleanup {
                operation: Box::new(operation),
                source: Box::new(Error::io(
                    "remove incomplete project",
                    root,
                    source,
                )),
            }),
        };
    }
    Ok(())
}

fn write_project(
    root: &Path,
    name: &str,
    package: &str,
    python: &str,
    command: Option<&str>,
) -> Result<()> {
    let module = root.join("src").join(package);
    let tests = root.join("tests");
    fs::create_dir_all(&module).map_err(|error| {
        Error::io("create package directory", &module, error)
    })?;
    fs::create_dir(&tests)
        .map_err(|error| Error::io("create test directory", &tests, error))?;

    let mut metadata = format!(
        "[project]\nname = \"{name}\"\nversion = \"0.1.0\"\nreadme = \"README.md\"\nrequires-python = \">={python}\"\ndependencies = []\n"
    );
    if let Some(command) = command {
        metadata.push_str(&format!(
            "\n[project.scripts]\n{command} = \"{package}.cli:main\"\n"
        ));
    }
    metadata.push_str(
        "\n[build-system]\nrequires = [\"uv_build>=0.9.26,<0.10.0\"]\nbuild-backend = \"uv_build\"\n",
    );
    if package != normalized_package(name) {
        metadata.push_str(&format!(
            "\n[tool.uv.build-backend]\nmodule-name = \"{package}\"\n"
        ));
    }
    metadata.push_str("\n[tool.pyfaro]\nschema-version = 1\n");
    write(&root.join("pyproject.toml"), &metadata)?;
    let mut readme = format!(
        "# {name}\n\nA {} created with Pyfaro.\n\n## Development\n\n```sh\npyfaro sync --offline\npyfaro run --locked --offline -- python -m unittest discover -s tests\nuv --no-python-downloads --offline build\n```\n",
        if command.is_some() {
            "command-line project"
        } else {
            "Python library"
        }
    );
    if let Some(command) = command {
        readme.push_str(&format!(
            "\nRun the installed command with `pyfaro run --locked --offline -- {command}`.\n"
        ));
    }
    write(&root.join("README.md"), &readme)?;
    write(&module.join("py.typed"), "")?;
    if command.is_some() {
        write(
            &module.join("__init__.py"),
            "\"\"\"Command-line application.\"\"\"\n",
        )?;
        write(
            &module.join("cli.py"),
            &format!(
                "\"\"\"Console entry point for {name}.\"\"\"\n\n\ndef main() -> int:\n    print(\"Hello from {name}!\")\n    return 0\n"
            ),
        )?;
        write(
            &tests.join("test_cli.py"),
            &format!(
                "\"\"\"Smoke test for the installed command.\"\"\"\n\nimport io\nimport unittest\nfrom contextlib import redirect_stdout\n\nfrom {package}.cli import main\n\n\nclass CommandTest(unittest.TestCase):\n    def test_main(self) -> None:\n        output = io.StringIO()\n        with redirect_stdout(output):\n            result = main()\n        self.assertEqual(result, 0)\n        self.assertEqual(output.getvalue(), \"Hello from {name}!\\n\")\n"
            ),
        )?;
    } else {
        write(
            &module.join("__init__.py"),
            &format!(
                "\"\"\"Public API for {name}.\"\"\"\n\n\ndef hello() -> str:\n    return \"Hello from {name}!\"\n"
            ),
        )?;
        write(
            &tests.join("test_library.py"),
            &format!(
                "\"\"\"Smoke test for the public API.\"\"\"\n\nimport unittest\n\nfrom {package} import hello\n\n\nclass LibraryTest(unittest.TestCase):\n    def test_hello(self) -> None:\n        self.assertEqual(hello(), \"Hello from {name}!\")\n"
            ),
        )?;
    }
    Ok(())
}

fn write(path: &Path, contents: &str) -> Result<()> {
    fs::write(path, contents).map_err(|error| Error::io("write", path, error))
}

fn normalized_package(name: &str) -> String {
    let mut package = String::new();
    for character in name.chars() {
        if matches!(character, '-' | '_' | '.') {
            if !package.ends_with('_') {
                package.push('_');
            }
        } else {
            package.push(character.to_ascii_lowercase());
        }
    }
    package
}

fn validate_distribution(name: &str) -> Result<()> {
    let bytes = name.as_bytes();
    if bytes.is_empty()
        || !bytes[0].is_ascii_alphanumeric()
        || !bytes[bytes.len() - 1].is_ascii_alphanumeric()
        || !bytes.iter().all(|byte| {
            byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.')
        })
        || is_windows_device_name(name)
    {
        return Err(Error::Project(
            "project name must contain ASCII letters, digits, dots, underscores, or hyphens, start/end with a letter or digit, and not be a reserved Windows device name".into(),
        ));
    }
    Ok(())
}

fn validate_package(package: &str) -> Result<()> {
    let bytes = package.as_bytes();
    let valid = !bytes.is_empty()
        && (bytes[0].is_ascii_alphabetic() || bytes[0] == b'_')
        && bytes
            .iter()
            .all(|byte| byte.is_ascii_alphanumeric() || *byte == b'_');
    let stdlib = include_str!("python_stdlib_modules.txt")
        .split_whitespace()
        .any(|name| name == package);
    if !valid
        || PYTHON_KEYWORDS.contains(&package)
        || matches!(package, "__main__" | "__pycache__")
        || stdlib
        || is_windows_device_name(package)
    {
        return Err(Error::Project(format!(
            "invalid Python package name: {package}; use an ASCII identifier that is not a keyword, standard-library module, Python special name, or reserved Windows device name"
        )));
    }
    Ok(())
}

fn validate_command(command: &str) -> Result<()> {
    let bytes = command.as_bytes();
    if bytes.is_empty()
        || !bytes[0].is_ascii_alphanumeric()
        || !bytes.iter().all(|byte| {
            byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_')
        })
        || is_windows_device_name(command)
    {
        return Err(Error::Project(format!(
            "invalid command name: {command}; use ASCII letters, digits, hyphens, or underscores and avoid reserved Windows device names"
        )));
    }
    Ok(())
}

fn is_windows_device_name(name: &str) -> bool {
    let stem = name.split('.').next().unwrap_or(name).to_ascii_uppercase();
    matches!(stem.as_str(), "CON" | "PRN" | "AUX" | "NUL")
        || (stem.len() == 4
            && (stem.starts_with("COM") || stem.starts_with("LPT"))
            && matches!(stem.as_bytes()[3], b'1'..=b'9'))
}

fn validate_python(version: &str) -> Result<()> {
    let Some(minor) = version.strip_prefix("3.") else {
        return Err(Error::Project(
            "Python version must be MAJOR.MINOR, for example 3.12".into(),
        ));
    };
    if minor.is_empty()
        || minor.starts_with('0')
        || !minor.bytes().all(|byte| byte.is_ascii_digit())
        || !matches!(minor.parse::<u8>(), Ok(8..=99))
    {
        return Err(Error::Project(
            "Python version must be MAJOR.MINOR, for example 3.12".into(),
        ));
    }
    Ok(())
}

const PYTHON_KEYWORDS: &[&str] = &[
    "False", "None", "True", "and", "as", "assert", "async", "await", "break",
    "class", "continue", "def", "del", "elif", "else", "except", "finally",
    "for", "from", "global", "if", "import", "in", "is", "lambda", "nonlocal",
    "not", "or", "pass", "raise", "return", "try", "while", "with", "yield",
    "match", "case", "type",
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn failed_write_removes_only_the_new_project() {
        let sandbox = tempfile::tempdir().unwrap();
        let parent_marker = sandbox.path().join("keep.txt");
        fs::write(&parent_marker, "keep").unwrap();
        let root = sandbox.path().join("new-project");
        let result = create_directory(&root, |root| {
            fs::write(root.join("partial.txt"), "partial").unwrap();
            Err(Error::Project("injected write failure".into()))
        });
        assert!(matches!(result, Err(Error::Project(_))));
        assert!(!root.exists());
        assert_eq!(fs::read_to_string(parent_marker).unwrap(), "keep");
    }
}
