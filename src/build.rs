use std::collections::BTreeMap;
use std::ffi::OsString;
use std::fs;
use std::io::{self, Write};
use std::path::{Component, Path, PathBuf};
use std::time::SystemTime;

use crate::cli::BuildArgs;
use crate::config::Config;
use crate::error::{Error, Result};
use crate::{process, project, uv};

/// Files of the output directory with the state `uv build` can change.
type Listing = BTreeMap<OsString, (SystemTime, u64)>;

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Kind {
    Sdist,
    Wheel,
}

impl Kind {
    fn label(self) -> &'static str {
        match self {
            Self::Sdist => "source distribution (.tar.gz)",
            Self::Wheel => "wheel (.whl)",
        }
    }
}

pub(crate) fn run(args: BuildArgs) -> Result<()> {
    let root = project::select(args.project.as_deref())?;
    let document = Config::load_document(&root)?;
    let name = document
        .get("project")
        .and_then(|project| project.as_table_like())
        .and_then(|project| project.get("name"))
        .and_then(|name| name.as_str())
        .map(normalize);
    let out_dir = match args.out_dir.as_deref() {
        Some(path) => project::target(path)?,
        None => root.join("dist"),
    };
    if args.clear && root.starts_with(&out_dir) {
        return Err(Error::Usage(format!(
            "--clear would delete the project: the output directory {} is or contains the project at {}; choose another --out-dir",
            out_dir.display(),
            root.display()
        )));
    }
    // Neither flag and both flags mean uv's default: a source distribution,
    // then a wheel built from it.
    let kinds = match (args.sdist, args.wheel) {
        (true, false) => vec![Kind::Sdist],
        (false, true) => vec![Kind::Wheel],
        _ => vec![Kind::Sdist, Kind::Wheel],
    };
    let before = if args.clear { Listing::new() } else { list(&out_dir)? };

    let cancelled = process::cancellation()?;
    let uv = uv::Uv::discover(args.uv, args.offline, &cancelled)?;
    let mut command = vec![OsString::from("build")];
    match (args.sdist, args.wheel) {
        (true, false) => command.push(OsString::from("--sdist")),
        (false, true) => command.push(OsString::from("--wheel")),
        _ => {}
    }
    if args.clear {
        command.push(OsString::from("--clear"));
    }
    command.push(OsString::from("--out-dir"));
    command.push(out_dir.as_os_str().to_owned());
    command.push(root.as_os_str().to_owned());
    uv.execute(&root, "build", &command, &cancelled)?;

    let artifacts = created(&before, &list(&out_dir)?, name.as_deref());
    for kind in kinds {
        if !artifacts.iter().any(|(found, _)| *found == kind) {
            return Err(Error::Project(format!(
                "uv build succeeded but wrote no {} for {} in {}; check the build backend in [build-system] of pyproject.toml",
                kind.label(),
                name.as_deref().unwrap_or("the project"),
                out_dir.display()
            )));
        }
    }
    let base = std::env::current_dir().and_then(fs::canonicalize).map_err(
        |error| Error::io("resolve", Path::new("current directory"), error),
    )?;
    let mut stdout = io::stdout().lock();
    for (_, file) in artifacts {
        let path = out_dir.join(file);
        writeln!(stdout, "built {}", relative(&path, &base).display())
            .map_err(Error::Output)?;
    }
    Ok(())
}

/// The regular files of `directory`; a missing directory is empty.
fn list(directory: &Path) -> Result<Listing> {
    let entries = match fs::read_dir(directory) {
        Ok(entries) => entries,
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            return Ok(Listing::new());
        }
        Err(error) => {
            return Err(Error::io("read output directory", directory, error));
        }
    };
    let mut listing = Listing::new();
    for entry in entries {
        let entry = entry.map_err(|error| {
            Error::io("read output directory", directory, error)
        })?;
        let path = entry.path();
        let metadata = fs::metadata(&path)
            .map_err(|error| Error::io("inspect", &path, error))?;
        if metadata.is_file() {
            let modified = metadata
                .modified()
                .map_err(|error| Error::io("inspect", &path, error))?;
            listing.insert(entry.file_name(), (modified, metadata.len()));
        }
    }
    Ok(listing)
}

/// Artifacts of the project that appeared or were rewritten, by file name.
fn created(
    before: &Listing,
    after: &Listing,
    name: Option<&str>,
) -> Vec<(Kind, OsString)> {
    let mut artifacts: Vec<_> = after
        .iter()
        .filter(|(file, state)| before.get(*file) != Some(state))
        .filter_map(|(file, _)| {
            let (kind, artifact) = classify(file.to_str()?)?;
            name.is_none_or(|name| normalize(artifact) == name)
                .then(|| (kind, file.clone()))
        })
        .collect();
    // Source distribution first, as uv builds them.
    artifacts.sort();
    artifacts
}

/// The kind of distribution a file name is, with its distribution name:
/// `{name}-{version}.tar.gz` (PEP 625) or `{name}-{version}-{tags}.whl`.
fn classify(file: &str) -> Option<(Kind, &str)> {
    if let Some(stem) = file.strip_suffix(".tar.gz") {
        return Some((Kind::Sdist, stem.rsplit_once('-')?.0));
    }
    let stem = file.strip_suffix(".whl")?;
    Some((Kind::Wheel, stem.split_once('-')?.0))
}

/// `path` relative to the directory `base` (both absolute), or `path` itself
/// when they share nothing but the root, for example on another drive.
fn relative(path: &Path, base: &Path) -> PathBuf {
    let path_parts: Vec<_> = path.components().collect();
    let base_parts: Vec<_> = base.components().collect();
    let shared = path_parts
        .iter()
        .zip(&base_parts)
        .take_while(|(left, right)| left == right)
        .count();
    let anchor = path_parts
        .iter()
        .take(shared)
        .filter(|part| matches!(part, Component::Normal(_)))
        .count();
    if anchor == 0 {
        return path.to_owned();
    }
    let mut relative = PathBuf::new();
    for _ in shared..base_parts.len() {
        relative.push("..");
    }
    relative.extend(&path_parts[shared..]);
    relative
}

/// Normalize a distribution name so that `Acme.Widget`, `acme-widget` and
/// `acme_widget` compare equal.
fn normalize(name: &str) -> String {
    let mut normalized = String::with_capacity(name.len());
    let mut separator = false;
    for character in name.chars() {
        if matches!(character, '-' | '_' | '.') {
            separator = true;
            continue;
        }
        if separator {
            normalized.push('_');
            separator = false;
        }
        normalized.extend(character.to_lowercase());
    }
    if separator {
        normalized.push('_');
    }
    normalized
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn names_are_normalized_like_pep_503() {
        assert_eq!(normalize("Acme.Widget"), "acme_widget");
        assert_eq!(normalize("acme--widget__2"), "acme_widget_2");
        assert_eq!(normalize("acme_widget"), normalize("acme-widget"));
    }

    #[test]
    fn paths_are_shown_relative_to_the_current_directory() {
        let base = Path::new("/work/project");
        let show = |path: &str| relative(Path::new(path), base);
        assert_eq!(show("/work/project/dist/a.whl"), Path::new("dist/a.whl"));
        assert_eq!(show("/work/dist/a.whl"), Path::new("../dist/a.whl"));
        assert_eq!(show("/work/project/a/b/c.whl"), Path::new("a/b/c.whl"));
        assert_eq!(show("/other/a.whl"), Path::new("/other/a.whl"));
    }

    #[test]
    fn artifacts_are_recognized_by_their_file_names() {
        assert!(matches!(
            classify("acme_widget-0.1.0.tar.gz"),
            Some((Kind::Sdist, "acme_widget"))
        ));
        assert!(matches!(
            classify("acme-widget-0.1.0.tar.gz"),
            Some((Kind::Sdist, "acme-widget"))
        ));
        assert!(matches!(
            classify("acme_widget-0.1.0-py3-none-any.whl"),
            Some((Kind::Wheel, "acme_widget"))
        ));
        assert!(classify(".gitignore").is_none());
        assert!(classify("notes.tar").is_none());
        assert!(classify("nohyphen.tar.gz").is_none());
    }

    #[test]
    fn only_created_or_rewritten_project_artifacts_are_reported() {
        let old = SystemTime::UNIX_EPOCH;
        let new = old + Duration::from_secs(5);
        let listing = |entries: &[(&str, SystemTime, u64)]| -> Listing {
            entries
                .iter()
                .map(|(file, time, size)| ((*file).into(), (*time, *size)))
                .collect()
        };
        let before = listing(&[
            ("acme_widget-0.1.0.tar.gz", old, 10),
            ("acme_widget-0.1.0-py3-none-any.whl", old, 20),
            ("other-1.0.tar.gz", old, 5),
        ]);
        let after = listing(&[
            ("acme_widget-0.1.0.tar.gz", old, 10),
            ("acme_widget-0.1.0-py3-none-any.whl", new, 20),
            ("acme_widget-0.2.0.tar.gz", new, 11),
            ("other-1.0.tar.gz", new, 5),
            (".gitignore", new, 1),
        ]);
        let found = created(&before, &after, Some("acme_widget"));
        let files: Vec<_> =
            found.iter().map(|(_, file)| file.to_str().unwrap()).collect();
        assert_eq!(
            files,
            ["acme_widget-0.2.0.tar.gz", "acme_widget-0.1.0-py3-none-any.whl"]
        );
        assert_eq!(created(&before, &after, None).len(), 3);
    }
}
