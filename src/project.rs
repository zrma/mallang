use std::{
    fs, io,
    path::{Path, PathBuf},
};

use serde::Deserialize;

pub const MANIFEST_FILE: &str = "mallang.toml";

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ProjectManifest {
    pub project: ProjectMetadata,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ProjectMetadata {
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Project {
    root: PathBuf,
    manifest_path: PathBuf,
    manifest: ProjectManifest,
    source_root: PathBuf,
    source_files: Vec<PathBuf>,
}

impl Project {
    pub fn name(&self) -> &str {
        &self.manifest.project.name
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn manifest_path(&self) -> &Path {
        &self.manifest_path
    }

    pub fn manifest(&self) -> &ProjectManifest {
        &self.manifest
    }

    pub fn source_root(&self) -> &Path {
        &self.source_root
    }

    pub fn source_files(&self) -> &[PathBuf] {
        &self.source_files
    }
}

#[derive(Debug)]
pub enum ProjectError {
    InspectInput {
        path: PathBuf,
        source: io::Error,
    },
    UnsupportedInput {
        path: PathBuf,
    },
    ManifestNotFound {
        start: PathBuf,
    },
    ReadManifest {
        path: PathBuf,
        source: io::Error,
    },
    ParseManifest {
        path: PathBuf,
        source: toml::de::Error,
    },
    InvalidProjectName {
        path: PathBuf,
        name: String,
    },
    ReservedProjectName {
        path: PathBuf,
        name: String,
    },
    MissingSourceRoot {
        path: PathBuf,
    },
    MissingEntrypoint {
        path: PathBuf,
    },
    ReadSourceDirectory {
        path: PathBuf,
        source: io::Error,
    },
}

impl std::fmt::Display for ProjectError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InspectInput { path, source } => {
                write!(
                    formatter,
                    "{}: failed to inspect project input: {source}",
                    path.display()
                )
            }
            Self::UnsupportedInput { path } => write!(
                formatter,
                "{}: project input must be a directory or {MANIFEST_FILE}",
                path.display()
            ),
            Self::ManifestNotFound { start } => write!(
                formatter,
                "{}: could not find {MANIFEST_FILE} in this directory or its parents",
                start.display()
            ),
            Self::ReadManifest { path, source } => {
                write!(
                    formatter,
                    "{}: failed to read manifest: {source}",
                    path.display()
                )
            }
            Self::ParseManifest { path, source } => {
                write!(formatter, "{}: invalid manifest: {source}", path.display())
            }
            Self::InvalidProjectName { path, name } => write!(
                formatter,
                "{}: invalid project name `{name}`; expected a lowercase path name",
                path.display()
            ),
            Self::ReservedProjectName { path, name } => write!(
                formatter,
                "{}: project name `{name}` is reserved for compiler-owned standard packages",
                path.display()
            ),
            Self::MissingSourceRoot { path } => {
                write!(
                    formatter,
                    "{}: project source directory is missing",
                    path.display()
                )
            }
            Self::MissingEntrypoint { path } => {
                write!(
                    formatter,
                    "{}: project entry source is missing",
                    path.display()
                )
            }
            Self::ReadSourceDirectory { path, source } => write!(
                formatter,
                "{}: failed to read project source directory: {source}",
                path.display()
            ),
        }
    }
}

impl std::error::Error for ProjectError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::InspectInput { source, .. }
            | Self::ReadManifest { source, .. }
            | Self::ReadSourceDirectory { source, .. } => Some(source),
            Self::ParseManifest { source, .. } => Some(source),
            Self::UnsupportedInput { .. }
            | Self::ManifestNotFound { .. }
            | Self::InvalidProjectName { .. }
            | Self::ReservedProjectName { .. }
            | Self::MissingSourceRoot { .. }
            | Self::MissingEntrypoint { .. } => None,
        }
    }
}

pub fn discover_project(input: impl AsRef<Path>) -> Result<Project, ProjectError> {
    let input = input.as_ref();
    let metadata = fs::metadata(input).map_err(|source| ProjectError::InspectInput {
        path: input.to_path_buf(),
        source,
    })?;
    let canonical_input = fs::canonicalize(input).map_err(|source| ProjectError::InspectInput {
        path: input.to_path_buf(),
        source,
    })?;

    let manifest_path = if metadata.is_dir() {
        find_nearest_manifest(&canonical_input).ok_or_else(|| ProjectError::ManifestNotFound {
            start: canonical_input.clone(),
        })?
    } else if metadata.is_file()
        && canonical_input.file_name() == Some(std::ffi::OsStr::new(MANIFEST_FILE))
    {
        canonical_input
    } else {
        return Err(ProjectError::UnsupportedInput {
            path: canonical_input,
        });
    };

    load_project(manifest_path)
}

fn find_nearest_manifest(start: &Path) -> Option<PathBuf> {
    start
        .ancestors()
        .map(|directory| directory.join(MANIFEST_FILE))
        .find(|candidate| candidate.is_file())
}

fn load_project(manifest_path: PathBuf) -> Result<Project, ProjectError> {
    let manifest_text =
        fs::read_to_string(&manifest_path).map_err(|source| ProjectError::ReadManifest {
            path: manifest_path.clone(),
            source,
        })?;
    let manifest: ProjectManifest =
        toml::from_str(&manifest_text).map_err(|source| ProjectError::ParseManifest {
            path: manifest_path.clone(),
            source,
        })?;

    if !is_valid_project_name(&manifest.project.name) {
        return Err(ProjectError::InvalidProjectName {
            path: manifest_path,
            name: manifest.project.name,
        });
    }
    if manifest.project.name == "std" {
        return Err(ProjectError::ReservedProjectName {
            path: manifest_path,
            name: manifest.project.name,
        });
    }

    let root = manifest_path
        .parent()
        .expect("a canonical manifest path has a parent")
        .to_path_buf();
    let source_root = root.join("src");
    if !source_root.is_dir() {
        return Err(ProjectError::MissingSourceRoot { path: source_root });
    }

    let entrypoint = source_root.join("main.mlg");
    if !entrypoint.is_file() {
        return Err(ProjectError::MissingEntrypoint { path: entrypoint });
    }

    let mut source_files = Vec::new();
    collect_source_files(&source_root, &mut source_files)?;
    source_files.sort_by(|left, right| {
        left.strip_prefix(&root)
            .expect("discovered source is inside the project root")
            .cmp(
                right
                    .strip_prefix(&root)
                    .expect("discovered source is inside the project root"),
            )
    });

    Ok(Project {
        root,
        manifest_path,
        manifest,
        source_root,
        source_files,
    })
}

fn collect_source_files(directory: &Path, sources: &mut Vec<PathBuf>) -> Result<(), ProjectError> {
    let entries = fs::read_dir(directory).map_err(|source| ProjectError::ReadSourceDirectory {
        path: directory.to_path_buf(),
        source,
    })?;

    for entry in entries {
        let entry = entry.map_err(|source| ProjectError::ReadSourceDirectory {
            path: directory.to_path_buf(),
            source,
        })?;
        let file_type = entry
            .file_type()
            .map_err(|source| ProjectError::ReadSourceDirectory {
                path: entry.path(),
                source,
            })?;

        if file_type.is_dir() {
            collect_source_files(&entry.path(), sources)?;
        } else if file_type.is_file() && entry.path().extension().is_some_and(|ext| ext == "mlg") {
            sources.push(entry.path());
        }
    }

    Ok(())
}

fn is_valid_project_name(name: &str) -> bool {
    let mut chars = name.chars();
    matches!(chars.next(), Some(first) if first.is_ascii_lowercase())
        && chars.all(|ch| ch.is_ascii_lowercase() || ch.is_ascii_digit() || ch == '-' || ch == '_')
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicU64, Ordering};

    use super::*;

    static NEXT_TEMP_ID: AtomicU64 = AtomicU64::new(0);

    struct TempProject {
        root: PathBuf,
    }

    impl TempProject {
        fn new(label: &str) -> Self {
            let id = NEXT_TEMP_ID.fetch_add(1, Ordering::Relaxed);
            let root = std::env::temp_dir().join(format!(
                "mallang-project-test-{}-{label}-{id}",
                std::process::id()
            ));
            fs::create_dir_all(&root).unwrap();
            let root = fs::canonicalize(root).unwrap();
            Self { root }
        }

        fn write(&self, path: &str, contents: &str) {
            let path = self.root.join(path);
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent).unwrap();
            }
            fs::write(path, contents).unwrap();
        }
    }

    impl Drop for TempProject {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.root);
        }
    }

    fn valid_project(label: &str) -> TempProject {
        let project = TempProject::new(label);
        project.write("mallang.toml", "[project]\nname = \"hello\"\n");
        project.write("src/main.mlg", "func main() {}\n");
        project
    }

    #[test]
    fn discovers_manifest_and_sorted_source_files() {
        let project = valid_project("sorted");
        project.write("src/greet/print.mlg", "func Print() {}\n");
        project.write("src/greet/model.mlg", "type Message struct {}\n");
        project.write("src/README.txt", "not a source\n");

        let discovered = discover_project(&project.root).unwrap();
        let relative_sources = discovered
            .source_files()
            .iter()
            .map(|path| path.strip_prefix(discovered.root()).unwrap())
            .collect::<Vec<_>>();

        assert_eq!(discovered.manifest().project.name, "hello");
        assert_eq!(
            relative_sources,
            vec![
                Path::new("src/greet/model.mlg"),
                Path::new("src/greet/print.mlg"),
                Path::new("src/main.mlg"),
            ]
        );
    }

    #[test]
    fn finds_the_nearest_manifest_from_a_nested_directory() {
        let outer = valid_project("nearest-outer");
        let inner_root = outer.root.join("src/inner");
        fs::create_dir_all(inner_root.join("src/deep")).unwrap();
        fs::write(
            inner_root.join("mallang.toml"),
            "[project]\nname = \"inner\"\n",
        )
        .unwrap();
        fs::write(inner_root.join("src/main.mlg"), "func main() {}\n").unwrap();

        let discovered = discover_project(inner_root.join("src/deep")).unwrap();

        assert_eq!(discovered.root(), inner_root);
        assert_eq!(discovered.manifest().project.name, "inner");
    }

    #[test]
    fn accepts_an_explicit_manifest_input() {
        let project = valid_project("manifest-input");

        let discovered = discover_project(project.root.join(MANIFEST_FILE)).unwrap();

        assert_eq!(discovered.root(), project.root);
    }

    #[test]
    fn rejects_non_manifest_file_inputs() {
        let project = valid_project("file-input");

        let error = discover_project(project.root.join("src/main.mlg")).unwrap_err();

        assert!(matches!(error, ProjectError::UnsupportedInput { .. }));
    }

    #[test]
    fn reports_when_no_parent_manifest_exists() {
        let directory = TempProject::new("missing-manifest");

        let error = discover_project(&directory.root).unwrap_err();

        assert!(matches!(error, ProjectError::ManifestNotFound { .. }));
    }

    #[test]
    fn rejects_unknown_manifest_fields() {
        let project = valid_project("unknown-field");
        project.write(
            "mallang.toml",
            "[project]\nname = \"hello\"\nunknown = true\n",
        );

        let error = discover_project(&project.root).unwrap_err();

        assert!(matches!(error, ProjectError::ParseManifest { .. }));
    }

    #[test]
    fn rejects_invalid_project_names() {
        let project = valid_project("invalid-name");
        project.write("mallang.toml", "[project]\nname = \"Hello World\"\n");

        let error = discover_project(&project.root).unwrap_err();

        assert!(matches!(error, ProjectError::InvalidProjectName { .. }));
    }

    #[test]
    fn rejects_reserved_standard_project_name() {
        let project = valid_project("reserved-name");
        project.write("mallang.toml", "[project]\nname = \"std\"\n");

        let error = discover_project(&project.root).unwrap_err();

        assert!(matches!(error, ProjectError::ReservedProjectName { .. }));
        assert!(error.to_string().contains("reserved"));
    }

    #[test]
    fn requires_the_source_root_and_entry_file() {
        let missing_source = TempProject::new("missing-source");
        missing_source.write("mallang.toml", "[project]\nname = \"hello\"\n");
        let source_error = discover_project(&missing_source.root).unwrap_err();

        assert!(matches!(
            source_error,
            ProjectError::MissingSourceRoot { .. }
        ));

        let missing_entry = TempProject::new("missing-entry");
        missing_entry.write("mallang.toml", "[project]\nname = \"hello\"\n");
        fs::create_dir_all(missing_entry.root.join("src")).unwrap();
        let entry_error = discover_project(&missing_entry.root).unwrap_err();

        assert!(matches!(
            entry_error,
            ProjectError::MissingEntrypoint { .. }
        ));
    }
}
