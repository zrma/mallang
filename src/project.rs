use std::{
    collections::{BTreeMap, BTreeSet},
    fs, io,
    path::{Path, PathBuf},
};

use serde::Deserialize;

pub const MANIFEST_FILE: &str = "mallang.toml";

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ProjectManifest {
    pub project: ProjectMetadata,
    #[serde(default)]
    pub dependencies: BTreeMap<String, PathDependency>,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ProjectMetadata {
    pub name: String,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct PathDependency {
    pub path: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Project {
    root_project: ProjectUnit,
    dependencies: Vec<ProjectUnit>,
}

#[derive(Debug, Clone, Copy)]
pub struct ProjectUnitRef<'a> {
    unit: &'a ProjectUnit,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ProjectUnit {
    root: PathBuf,
    manifest_path: PathBuf,
    manifest: ProjectManifest,
    source_root: PathBuf,
    source_files: Vec<PathBuf>,
    test_root: PathBuf,
    direct_dependencies: BTreeSet<String>,
}

impl Project {
    pub fn name(&self) -> &str {
        self.root_project.name()
    }

    pub fn root(&self) -> &Path {
        &self.root_project.root
    }

    pub fn manifest_path(&self) -> &Path {
        &self.root_project.manifest_path
    }

    pub fn manifest(&self) -> &ProjectManifest {
        &self.root_project.manifest
    }

    pub fn source_root(&self) -> &Path {
        &self.root_project.source_root
    }

    pub fn source_files(&self) -> &[PathBuf] {
        &self.root_project.source_files
    }

    pub fn test_root(&self) -> &Path {
        &self.root_project.test_root
    }

    pub fn has_entrypoint(&self) -> bool {
        self.root_project.entrypoint().is_file()
    }

    pub fn require_entrypoint(&self) -> Result<(), ProjectError> {
        if self.has_entrypoint() {
            return Ok(());
        }
        Err(ProjectError::MissingEntrypoint {
            path: self.root_project.entrypoint(),
        })
    }

    pub fn dependency_names(&self) -> impl Iterator<Item = &str> {
        self.dependencies.iter().map(ProjectUnit::name)
    }

    pub fn compiler_units(&self) -> impl Iterator<Item = ProjectUnitRef<'_>> {
        std::iter::once(ProjectUnitRef {
            unit: &self.root_project,
        })
        .chain(self.dependencies.iter().map(|unit| ProjectUnitRef { unit }))
    }

    pub fn diagnostic_path(&self, path: &Path) -> PathBuf {
        for dependency in &self.dependencies {
            if let Ok(relative) = path.strip_prefix(&dependency.root) {
                return PathBuf::from(dependency.name()).join(relative);
            }
        }
        if let Ok(relative) = path.strip_prefix(&self.root_project.root) {
            return relative.to_path_buf();
        }
        path.to_path_buf()
    }

    pub fn compilation_source_files(&self) -> Vec<&PathBuf> {
        let mut files = Vec::new();
        for dependency in &self.dependencies {
            let entrypoint = dependency.entrypoint();
            files.extend(
                dependency
                    .source_files
                    .iter()
                    .filter(|path| path.as_path() != entrypoint),
            );
        }
        files.extend(self.root_project.source_files.iter());
        files
    }

    pub fn discover_test_files(&self) -> Result<Vec<PathBuf>, ProjectError> {
        let mut test_files = Vec::new();
        match fs::metadata(&self.root_project.test_root) {
            Ok(metadata) if !metadata.is_dir() => {
                return Err(ProjectError::InvalidTestRoot {
                    path: self.root_project.test_root.clone(),
                });
            }
            Ok(_) => {
                collect_source_files(&self.root_project.test_root, &mut test_files)?;
                sort_project_files(&self.root_project.root, &mut test_files);
            }
            Err(source) if source.kind() == io::ErrorKind::NotFound => {}
            Err(source) => {
                return Err(ProjectError::ReadSourceDirectory {
                    path: self.root_project.test_root.clone(),
                    source,
                });
            }
        }

        Ok(test_files)
    }

    pub(crate) fn source_directory_identity<'a, 'b>(
        &'a self,
        directory: &'b Path,
    ) -> Option<(&'a str, &'b Path)> {
        if let Ok(relative) = directory.strip_prefix(&self.root_project.test_root) {
            return Some((self.root_project.name(), relative));
        }
        if let Ok(relative) = directory.strip_prefix(&self.root_project.source_root) {
            return Some((self.root_project.name(), relative));
        }
        for dependency in &self.dependencies {
            if let Ok(relative) = directory.strip_prefix(&dependency.source_root) {
                return Some((dependency.name(), relative));
            }
        }
        None
    }

    pub(crate) fn is_test_source_path(&self, path: &Path) -> bool {
        path.starts_with(&self.root_project.test_root)
    }

    pub(crate) fn allows_project_import(&self, from: &str, target: &str) -> bool {
        if from == target {
            return true;
        }
        self.unit(from)
            .is_some_and(|unit| unit.direct_dependencies.contains(target))
    }

    fn unit(&self, name: &str) -> Option<&ProjectUnit> {
        if self.root_project.name() == name {
            return Some(&self.root_project);
        }
        self.dependencies
            .iter()
            .find(|project| project.name() == name)
    }
}

impl<'a> ProjectUnitRef<'a> {
    pub fn name(self) -> &'a str {
        self.unit.name()
    }

    pub fn source_root(self) -> &'a Path {
        &self.unit.source_root
    }

    pub fn direct_dependencies(self) -> impl Iterator<Item = &'a str> + 'a {
        self.unit.direct_dependencies.iter().map(String::as_str)
    }
}

impl ProjectUnit {
    fn name(&self) -> &str {
        &self.manifest.project.name
    }

    fn entrypoint(&self) -> PathBuf {
        self.source_root.join("main.mlg")
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
    InvalidTestRoot {
        path: PathBuf,
    },
    InvalidDependencyPath {
        manifest: PathBuf,
        dependency: String,
        path: String,
    },
    ResolveDependency {
        manifest: PathBuf,
        dependency: String,
        path: PathBuf,
        source: io::Error,
    },
    DependencyNotDirectory {
        manifest: PathBuf,
        dependency: String,
        path: PathBuf,
    },
    DependencyNameMismatch {
        manifest: PathBuf,
        dependency: String,
        actual: String,
    },
    ProjectNameCollision {
        name: String,
        first: PathBuf,
        second: PathBuf,
    },
    DependencyCycle {
        projects: Vec<String>,
    },
    OverlappingProjectRoot {
        project: String,
        root: PathBuf,
        owner: String,
        boundary: PathBuf,
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
            Self::InvalidTestRoot { path } => write!(
                formatter,
                "{}: project test root must be a directory when present",
                path.display()
            ),
            Self::InvalidDependencyPath {
                manifest,
                dependency,
                path,
            } => write!(
                formatter,
                "{}: dependency `{dependency}` path `{path}` must be a non-empty relative directory path",
                manifest.display()
            ),
            Self::ResolveDependency {
                manifest,
                dependency,
                path,
                source,
            } => write!(
                formatter,
                "{}: failed to resolve dependency `{dependency}` at {}: {source}",
                manifest.display(),
                path.display()
            ),
            Self::DependencyNotDirectory {
                manifest,
                dependency,
                path,
            } => write!(
                formatter,
                "{}: dependency `{dependency}` path {} must resolve to a project directory",
                manifest.display(),
                path.display()
            ),
            Self::DependencyNameMismatch {
                manifest,
                dependency,
                actual,
            } => write!(
                formatter,
                "{}: dependency key `{dependency}` does not match target project name `{actual}`",
                manifest.display()
            ),
            Self::ProjectNameCollision {
                name,
                first,
                second,
            } => write!(
                formatter,
                "project name `{name}` resolves to multiple manifests: {} and {}",
                first.display(),
                second.display()
            ),
            Self::DependencyCycle { projects } => {
                write!(formatter, "project dependency cycle: {}", projects.join(" -> "))
            }
            Self::OverlappingProjectRoot {
                project,
                root,
                owner,
                boundary,
            } => write!(
                formatter,
                "project `{project}` root {} overlaps project `{owner}` source boundary {}",
                root.display(),
                boundary.display()
            ),
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
            | Self::ResolveDependency { source, .. }
            | Self::ReadSourceDirectory { source, .. } => Some(source),
            Self::ParseManifest { source, .. } => Some(source),
            Self::UnsupportedInput { .. }
            | Self::ManifestNotFound { .. }
            | Self::InvalidProjectName { .. }
            | Self::ReservedProjectName { .. }
            | Self::MissingSourceRoot { .. }
            | Self::MissingEntrypoint { .. }
            | Self::InvalidTestRoot { .. }
            | Self::InvalidDependencyPath { .. }
            | Self::DependencyNotDirectory { .. }
            | Self::DependencyNameMismatch { .. }
            | Self::ProjectNameCollision { .. }
            | Self::DependencyCycle { .. }
            | Self::OverlappingProjectRoot { .. } => None,
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

    let manifest_path =
        fs::canonicalize(&manifest_path).map_err(|source| ProjectError::InspectInput {
            path: manifest_path,
            source,
        })?;
    ProjectLoader::new().load(manifest_path)
}

fn find_nearest_manifest(start: &Path) -> Option<PathBuf> {
    start
        .ancestors()
        .map(|directory| directory.join(MANIFEST_FILE))
        .find(|candidate| candidate.is_file())
}

struct ProjectLoader {
    completed: BTreeMap<PathBuf, ProjectUnit>,
    order: Vec<PathBuf>,
    visiting: Vec<(PathBuf, String)>,
    names: BTreeMap<String, PathBuf>,
}

impl ProjectLoader {
    fn new() -> Self {
        Self {
            completed: BTreeMap::new(),
            order: Vec::new(),
            visiting: Vec::new(),
            names: BTreeMap::new(),
        }
    }

    fn load(mut self, root_manifest: PathBuf) -> Result<Project, ProjectError> {
        self.visit(root_manifest.clone())?;
        let root_project = self
            .completed
            .remove(&root_manifest)
            .expect("the root project is loaded by dependency traversal");
        let mut dependencies = Vec::new();
        for manifest in self.order {
            if manifest == root_manifest {
                continue;
            }
            dependencies.push(
                self.completed
                    .remove(&manifest)
                    .expect("dependency order only contains loaded projects"),
            );
        }
        let project = Project {
            root_project,
            dependencies,
        };
        validate_project_boundaries(&project)?;
        Ok(project)
    }

    fn visit(&mut self, manifest_path: PathBuf) -> Result<String, ProjectError> {
        if let Some(project) = self.completed.get(&manifest_path) {
            return Ok(project.name().to_string());
        }
        if let Some(cycle_start) = self
            .visiting
            .iter()
            .position(|(path, _)| path == &manifest_path)
        {
            let mut projects = self.visiting[cycle_start..]
                .iter()
                .map(|(_, name)| name.clone())
                .collect::<Vec<_>>();
            projects.push(self.visiting[cycle_start].1.clone());
            return Err(ProjectError::DependencyCycle { projects });
        }

        let mut project = load_project_unit(manifest_path.clone())?;
        if let Some(existing) = self.names.get(project.name()) {
            if existing != &manifest_path {
                return Err(ProjectError::ProjectNameCollision {
                    name: project.name().to_string(),
                    first: existing.clone(),
                    second: manifest_path,
                });
            }
        } else {
            self.names
                .insert(project.name().to_string(), manifest_path.clone());
        }

        self.visiting
            .push((manifest_path.clone(), project.name().to_string()));
        let dependencies = project.manifest.dependencies.clone();
        for (dependency, specification) in dependencies {
            let dependency_manifest = resolve_dependency_manifest(
                &project.manifest_path,
                &dependency,
                &specification.path,
            )?;
            let actual = self.visit(dependency_manifest)?;
            if actual != dependency {
                return Err(ProjectError::DependencyNameMismatch {
                    manifest: project.manifest_path.clone(),
                    dependency,
                    actual,
                });
            }
            project.direct_dependencies.insert(actual);
        }
        let popped = self
            .visiting
            .pop()
            .expect("visiting stack contains the current project");
        debug_assert_eq!(popped.0, manifest_path);

        let name = project.name().to_string();
        self.completed.insert(manifest_path.clone(), project);
        self.order.push(manifest_path);
        Ok(name)
    }
}

fn resolve_dependency_manifest(
    manifest: &Path,
    dependency: &str,
    path: &str,
) -> Result<PathBuf, ProjectError> {
    let relative = Path::new(path);
    if path.is_empty() || !relative.is_relative() {
        return Err(ProjectError::InvalidDependencyPath {
            manifest: manifest.to_path_buf(),
            dependency: dependency.to_string(),
            path: path.to_string(),
        });
    }
    let candidate = manifest
        .parent()
        .expect("a manifest path has a parent")
        .join(relative);
    let root = fs::canonicalize(&candidate).map_err(|source| ProjectError::ResolveDependency {
        manifest: manifest.to_path_buf(),
        dependency: dependency.to_string(),
        path: candidate.clone(),
        source,
    })?;
    if !root.is_dir() {
        return Err(ProjectError::DependencyNotDirectory {
            manifest: manifest.to_path_buf(),
            dependency: dependency.to_string(),
            path: root,
        });
    }
    let dependency_manifest = root.join(MANIFEST_FILE);
    fs::canonicalize(&dependency_manifest).map_err(|source| ProjectError::ResolveDependency {
        manifest: manifest.to_path_buf(),
        dependency: dependency.to_string(),
        path: dependency_manifest,
        source,
    })
}

fn load_project_unit(manifest_path: PathBuf) -> Result<ProjectUnit, ProjectError> {
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

    let mut source_files = Vec::new();
    collect_source_files(&source_root, &mut source_files)?;
    sort_project_files(&root, &mut source_files);

    let test_root = root.join("tests");
    Ok(ProjectUnit {
        root,
        manifest_path,
        manifest,
        source_root,
        source_files,
        test_root,
        direct_dependencies: BTreeSet::new(),
    })
}

fn validate_project_boundaries(project: &Project) -> Result<(), ProjectError> {
    let projects = std::iter::once(&project.root_project)
        .chain(project.dependencies.iter())
        .collect::<Vec<_>>();
    for owner in &projects {
        for candidate in &projects {
            if owner.manifest_path == candidate.manifest_path {
                continue;
            }
            for boundary in [&owner.source_root, &owner.test_root] {
                if candidate.root.starts_with(boundary) {
                    return Err(ProjectError::OverlappingProjectRoot {
                        project: candidate.name().to_string(),
                        root: candidate.root.clone(),
                        owner: owner.name().to_string(),
                        boundary: boundary.clone(),
                    });
                }
            }
        }
    }
    Ok(())
}

fn sort_project_files(root: &Path, files: &mut [PathBuf]) {
    files.sort_by(|left, right| {
        left.strip_prefix(root)
            .expect("discovered source is inside the project root")
            .cmp(
                right
                    .strip_prefix(root)
                    .expect("discovered source is inside the project root"),
            )
    });
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
    fn discovers_optional_sorted_test_files() {
        let project = valid_project("sorted-tests");
        project.write("tests/stats/zeta.mlg", "package stats\n");
        project.write("tests/main_test.mlg", "package main\n");
        project.write("tests/stats/alpha.mlg", "package stats\n");
        project.write("tests/README.txt", "not a source\n");

        let discovered = discover_project(&project.root).unwrap();
        let test_files = discovered.discover_test_files().unwrap();
        let relative_tests = test_files
            .iter()
            .map(|path| path.strip_prefix(discovered.root()).unwrap())
            .collect::<Vec<_>>();

        assert_eq!(discovered.test_root(), project.root.join("tests"));
        assert_eq!(
            relative_tests,
            vec![
                Path::new("tests/main_test.mlg"),
                Path::new("tests/stats/alpha.mlg"),
                Path::new("tests/stats/zeta.mlg"),
            ]
        );
    }

    #[test]
    fn treats_a_missing_test_root_as_an_empty_test_set() {
        let project = valid_project("missing-tests");

        let discovered = discover_project(&project.root).unwrap();

        assert!(discovered.discover_test_files().unwrap().is_empty());
        assert_eq!(discovered.test_root(), project.root.join("tests"));
    }

    #[test]
    fn rejects_a_non_directory_test_root_when_tests_are_discovered() {
        let project = valid_project("invalid-tests");
        project.write("tests", "not a directory\n");

        let discovered = discover_project(&project.root).unwrap();
        let error = discovered.discover_test_files().unwrap_err();

        assert!(matches!(error, ProjectError::InvalidTestRoot { .. }));
        assert!(error.to_string().contains("test root must be a directory"));
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
    fn requires_the_source_root_and_allows_library_projects() {
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
        let library = discover_project(&missing_entry.root).unwrap();
        let entry_error = library.require_entrypoint().unwrap_err();

        assert!(!library.has_entrypoint());
        assert!(matches!(
            entry_error,
            ProjectError::MissingEntrypoint { .. }
        ));
    }

    #[test]
    fn discovers_dependency_first_transitive_diamond_sources() {
        let project = valid_project("dependency-order");
        project.write(
            "mallang.toml",
            "[project]\nname = \"app\"\n\n[dependencies]\nshared = { path = \"deps/shared\" }\ntext = { path = \"deps/text\" }\n",
        );
        project.write("deps/shared/mallang.toml", "[project]\nname = \"shared\"\n");
        project.write(
            "deps/shared/src/shared.mlg",
            "package main\npub func Value() int { return 42 }\n",
        );
        project.write(
            "deps/text/mallang.toml",
            "[project]\nname = \"text\"\n\n[dependencies]\nshared = { path = \"../shared\" }\n",
        );
        project.write(
            "deps/text/src/text.mlg",
            "package main\nimport \"shared\"\npub func Read() int { return shared.Value() }\n",
        );
        project.write(
            "deps/text/src/main.mlg",
            "package main\nfunc main() { print(Read()) }\n",
        );

        let discovered = discover_project(&project.root).unwrap();
        let dependencies = discovered.dependency_names().collect::<Vec<_>>();
        let compiler_units = discovered
            .compiler_units()
            .map(|unit| (unit.name(), unit.direct_dependencies().collect::<Vec<_>>()))
            .collect::<Vec<_>>();
        let sources = discovered
            .compilation_source_files()
            .into_iter()
            .map(|path| path.strip_prefix(discovered.root()).unwrap())
            .collect::<Vec<_>>();

        assert_eq!(dependencies, ["shared", "text"]);
        assert_eq!(
            compiler_units,
            [
                ("app", vec!["shared", "text"]),
                ("shared", vec![]),
                ("text", vec!["shared"]),
            ]
        );
        assert_eq!(
            sources,
            [
                Path::new("deps/shared/src/shared.mlg"),
                Path::new("deps/text/src/text.mlg"),
                Path::new("src/main.mlg"),
            ]
        );
        assert_eq!(
            discovered.diagnostic_path(&project.root.join("deps/text/src/text.mlg")),
            Path::new("text/src/text.mlg")
        );
        assert_eq!(
            discovered.diagnostic_path(&project.root.join("src/main.mlg")),
            Path::new("src/main.mlg")
        );
    }

    #[test]
    fn rejects_invalid_dependency_paths_and_name_mismatches() {
        let absolute = valid_project("absolute-dependency");
        absolute.write(
            "mallang.toml",
            "[project]\nname = \"app\"\n\n[dependencies]\ntext = { path = \"/tmp/text\" }\n",
        );
        assert!(matches!(
            discover_project(&absolute.root).unwrap_err(),
            ProjectError::InvalidDependencyPath { .. }
        ));

        let empty = valid_project("empty-dependency");
        empty.write(
            "mallang.toml",
            "[project]\nname = \"app\"\n\n[dependencies]\ntext = { path = \"\" }\n",
        );
        assert!(matches!(
            discover_project(&empty.root).unwrap_err(),
            ProjectError::InvalidDependencyPath { .. }
        ));

        let unknown = valid_project("unknown-dependency-field");
        unknown.write(
            "mallang.toml",
            "[project]\nname = \"app\"\n\n[dependencies]\ntext = { path = \"deps/text\", version = \"1\" }\n",
        );
        assert!(matches!(
            discover_project(&unknown.root).unwrap_err(),
            ProjectError::ParseManifest { .. }
        ));

        let mismatch = valid_project("dependency-name-mismatch");
        mismatch.write(
            "mallang.toml",
            "[project]\nname = \"app\"\n\n[dependencies]\nwrong = { path = \"deps/text\" }\n",
        );
        mismatch.write("deps/text/mallang.toml", "[project]\nname = \"text\"\n");
        mismatch.write("deps/text/src/text.mlg", "package main\n");
        assert!(matches!(
            discover_project(&mismatch.root).unwrap_err(),
            ProjectError::DependencyNameMismatch { .. }
        ));
    }

    #[test]
    fn rejects_dependency_cycles_and_project_name_collisions() {
        let cycle = valid_project("dependency-cycle");
        cycle.write(
            "mallang.toml",
            "[project]\nname = \"app\"\n\n[dependencies]\ntext = { path = \"deps/text\" }\n",
        );
        cycle.write(
            "deps/text/mallang.toml",
            "[project]\nname = \"text\"\n\n[dependencies]\napp = { path = \"../..\" }\n",
        );
        cycle.write("deps/text/src/text.mlg", "package main\n");
        let error = discover_project(&cycle.root).unwrap_err();
        assert!(matches!(error, ProjectError::DependencyCycle { .. }));
        assert_eq!(
            error.to_string(),
            "project dependency cycle: app -> text -> app"
        );

        let collision = valid_project("dependency-name-collision");
        collision.write(
            "mallang.toml",
            "[project]\nname = \"app\"\n\n[dependencies]\nleft = { path = \"deps/left\" }\nright = { path = \"deps/right\" }\n",
        );
        for directory in ["left", "right"] {
            collision.write(
                &format!("deps/{directory}/mallang.toml"),
                &format!("[project]\nname = \"{directory}\"\n\n[dependencies]\nshared = {{ path = \"../{directory}-shared\" }}\n"),
            );
            collision.write(
                &format!("deps/{directory}/src/{directory}.mlg"),
                "package main\n",
            );
            collision.write(
                &format!("deps/{directory}-shared/mallang.toml"),
                "[project]\nname = \"shared\"\n",
            );
            collision.write(
                &format!("deps/{directory}-shared/src/shared.mlg"),
                "package main\n",
            );
        }
        assert!(matches!(
            discover_project(&collision.root).unwrap_err(),
            ProjectError::ProjectNameCollision { .. }
        ));
    }

    #[test]
    fn rejects_dependency_projects_inside_source_boundaries() {
        let project = valid_project("overlapping-dependency");
        project.write(
            "mallang.toml",
            "[project]\nname = \"app\"\n\n[dependencies]\ninside = { path = \"src/inside\" }\n",
        );
        project.write("src/inside/mallang.toml", "[project]\nname = \"inside\"\n");
        project.write("src/inside/src/library.mlg", "package main\n");

        assert!(matches!(
            discover_project(&project.root).unwrap_err(),
            ProjectError::OverlappingProjectRoot { .. }
        ));
    }
}
