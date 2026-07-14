use std::{
    collections::{BTreeMap, BTreeSet},
    path::{Component, Path},
};

use crate::{
    ast::{Program, SourceUnit, Visibility},
    project::Project,
    source::SourceMap,
    standard::{self, STANDARD_PREFIX},
    token::{SourceId, Span},
};

pub const STANDALONE_PACKAGE_PATH: &str = "standalone";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PackageGraph {
    packages: BTreeMap<String, Package>,
    source_packages: BTreeMap<SourceId, String>,
    build_order: Vec<String>,
}

impl PackageGraph {
    pub fn packages(&self) -> &BTreeMap<String, Package> {
        &self.packages
    }

    pub fn package(&self, path: &str) -> Option<&Package> {
        self.packages.get(path)
    }

    pub fn package_for_source(&self, source: SourceId) -> Option<&Package> {
        self.source_packages
            .get(&source)
            .and_then(|path| self.packages.get(path))
    }

    pub fn build_order(&self) -> &[String] {
        &self.build_order
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Package {
    pub path: String,
    pub name: String,
    pub source_ids: Vec<SourceId>,
    pub imports: Vec<PackageImport>,
    pub declarations: BTreeMap<String, PackageDeclaration>,
    pub methods: BTreeMap<String, BTreeMap<String, PackageDeclaration>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PackageImport {
    pub path: String,
    pub qualifier: String,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PackageDeclaration {
    pub name: String,
    pub kind: PackageDeclarationKind,
    pub type_params: Vec<String>,
    pub visibility: Visibility,
    pub span: Span,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PackageDeclarationKind {
    Struct,
    Opaque,
    Enum,
    Function,
    Method,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PackageError {
    pub message: String,
    pub span: Option<Span>,
}

impl PackageError {
    fn new(message: impl Into<String>, span: Option<Span>) -> Self {
        Self {
            message: message.into(),
            span,
        }
    }
}

impl std::fmt::Display for PackageError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(span) = self.span {
            write!(
                formatter,
                "{} at {}..{}",
                self.message, span.start, span.end
            )
        } else {
            formatter.write_str(&self.message)
        }
    }
}

impl std::error::Error for PackageError {}

struct PendingImport {
    from: String,
    import: PackageImport,
}

pub fn build_package_graph(
    project: &Project,
    sources: &SourceMap,
    program: &Program,
) -> Result<PackageGraph, PackageError> {
    let mut packages = BTreeMap::<String, Package>::new();
    let mut source_packages = BTreeMap::new();
    let mut pending_imports = Vec::new();

    for unit in &program.source_units {
        let source_id = unit.span.source;
        let source = sources.file(source_id).ok_or_else(|| {
            PackageError::new(
                format!("source ID {} is not registered", source_id.index()),
                None,
            )
        })?;
        let identity = package_identity(project, source.path())
            .map_err(|message| PackageError::new(message, Some(unit.span)))?;
        let declaration = unit.package.as_ref().ok_or_else(|| {
            PackageError::new(
                format!(
                    "source in package `{}` must declare `package {}`",
                    identity.path, identity.name
                ),
                Some(unit.span),
            )
        })?;
        if declaration.name != identity.name {
            return Err(PackageError::new(
                format!(
                    "package `{}` does not match source directory package `{}`",
                    declaration.name, identity.name
                ),
                Some(declaration.span),
            ));
        }
        if source_packages
            .insert(source_id, identity.path.clone())
            .is_some()
        {
            return Err(PackageError::new(
                format!("source ID {} appears more than once", source_id.index()),
                Some(unit.span),
            ));
        }

        let package = packages
            .entry(identity.path.clone())
            .or_insert_with(|| Package {
                path: identity.path.clone(),
                name: identity.name.clone(),
                source_ids: Vec::new(),
                imports: Vec::new(),
                declarations: BTreeMap::new(),
                methods: BTreeMap::new(),
            });
        package.source_ids.push(source_id);
        collect_file_imports(unit, &identity.path, &mut pending_imports)?;
    }

    connect_standard_packages(&mut packages, &pending_imports)?;
    connect_imports(&mut packages, pending_imports)?;
    collect_declarations(&mut packages, &source_packages, program, true)?;
    let build_order = topological_order(&packages)?;

    Ok(PackageGraph {
        packages,
        source_packages,
        build_order,
    })
}

pub fn build_standalone_package_graph(
    sources: &SourceMap,
    program: &Program,
) -> Result<PackageGraph, PackageError> {
    let mut packages = BTreeMap::from([(
        STANDALONE_PACKAGE_PATH.to_string(),
        Package {
            path: STANDALONE_PACKAGE_PATH.to_string(),
            name: "main".to_string(),
            source_ids: Vec::new(),
            imports: Vec::new(),
            declarations: BTreeMap::new(),
            methods: BTreeMap::new(),
        },
    )]);
    let mut source_packages = BTreeMap::new();
    let mut pending_imports = Vec::new();

    for unit in &program.source_units {
        let source_id = unit.span.source;
        if sources.file(source_id).is_none() {
            return Err(PackageError::new(
                format!("source ID {} is not registered", source_id.index()),
                None,
            ));
        }
        if source_packages
            .insert(source_id, STANDALONE_PACKAGE_PATH.to_string())
            .is_some()
        {
            return Err(PackageError::new(
                format!("source ID {} appears more than once", source_id.index()),
                Some(unit.span),
            ));
        }
        packages
            .get_mut(STANDALONE_PACKAGE_PATH)
            .expect("the standalone root package is initialized")
            .source_ids
            .push(source_id);
        collect_file_imports(unit, STANDALONE_PACKAGE_PATH, &mut pending_imports)?;
    }

    connect_standard_packages(&mut packages, &pending_imports)?;
    connect_imports(&mut packages, pending_imports)?;
    collect_declarations(&mut packages, &source_packages, program, false)?;
    let build_order = topological_order(&packages)?;

    Ok(PackageGraph {
        packages,
        source_packages,
        build_order,
    })
}

struct PackageIdentity {
    path: String,
    name: String,
}

fn package_identity(project: &Project, source_path: &Path) -> Result<PackageIdentity, String> {
    let source_directory = source_path.parent().ok_or_else(|| {
        format!(
            "{}: source path has no parent directory",
            source_path.display()
        )
    })?;
    let relative = source_directory
        .strip_prefix(project.source_root())
        .map_err(|_| {
            format!(
                "{}: source is outside the project source directory",
                source_path.display()
            )
        })?;

    if relative.as_os_str().is_empty() {
        return Ok(PackageIdentity {
            path: project.name().to_string(),
            name: "main".to_string(),
        });
    }

    let mut segments = Vec::new();
    for component in relative.components() {
        let Component::Normal(segment) = component else {
            return Err(format!(
                "{}: package directory has an invalid path component",
                source_path.display()
            ));
        };
        let segment = segment.to_str().ok_or_else(|| {
            format!(
                "{}: package directory is not valid UTF-8",
                source_path.display()
            )
        })?;
        segments.push(segment.to_string());
    }

    let name = segments
        .last()
        .expect("a non-root package has at least one path segment")
        .clone();
    Ok(PackageIdentity {
        path: format!("{}/{}", project.name(), segments.join("/")),
        name,
    })
}

fn collect_file_imports(
    unit: &SourceUnit,
    package_path: &str,
    pending: &mut Vec<PendingImport>,
) -> Result<(), PackageError> {
    let mut paths = BTreeSet::new();
    let mut qualifiers = BTreeMap::<String, String>::new();

    for import in &unit.imports {
        let qualifier = import
            .path
            .rsplit_once('/')
            .map_or(import.path.as_str(), |(_, qualifier)| qualifier);
        if import.path.is_empty()
            || import.path.starts_with('/')
            || import.path.ends_with('/')
            || import.path.split('/').any(str::is_empty)
            || !is_identifier(qualifier)
        {
            return Err(PackageError::new(
                format!("invalid import path `{}`", import.path),
                Some(import.span),
            ));
        }
        if !paths.insert(import.path.as_str()) {
            return Err(PackageError::new(
                format!("duplicate import `{}`", import.path),
                Some(import.span),
            ));
        }
        if let Some(existing) = qualifiers.insert(qualifier.to_string(), import.path.clone()) {
            return Err(PackageError::new(
                format!(
                    "imports `{existing}` and `{}` use the same qualifier `{qualifier}`",
                    import.path
                ),
                Some(import.span),
            ));
        }

        pending.push(PendingImport {
            from: package_path.to_string(),
            import: PackageImport {
                path: import.path.clone(),
                qualifier: qualifier.to_string(),
                span: import.span,
            },
        });
    }

    Ok(())
}

fn connect_imports(
    packages: &mut BTreeMap<String, Package>,
    pending: Vec<PendingImport>,
) -> Result<(), PackageError> {
    for pending_import in pending {
        if !packages.contains_key(&pending_import.import.path) {
            let message = if pending_import.import.path.starts_with(STANDARD_PREFIX) {
                format!("unknown standard package `{}`", pending_import.import.path)
            } else {
                format!("unresolved import `{}`", pending_import.import.path)
            };
            return Err(PackageError::new(message, Some(pending_import.import.span)));
        }

        let package = packages
            .get_mut(&pending_import.from)
            .expect("an import source package was collected first");
        if package
            .imports
            .iter()
            .all(|import| import.path != pending_import.import.path)
        {
            package.imports.push(pending_import.import);
        }
    }

    for package in packages.values_mut() {
        package
            .imports
            .sort_by(|left, right| left.path.cmp(&right.path));
    }
    Ok(())
}

fn connect_standard_packages(
    packages: &mut BTreeMap<String, Package>,
    pending: &[PendingImport],
) -> Result<(), PackageError> {
    for pending_import in pending {
        let Some(package) =
            standard::package(&pending_import.import.path, pending_import.import.span)
        else {
            continue;
        };
        if packages
            .get(&pending_import.import.path)
            .is_some_and(|existing| !existing.source_ids.is_empty())
        {
            return Err(PackageError::new(
                format!(
                    "source package `{}` shadows a reserved standard package",
                    pending_import.import.path
                ),
                Some(pending_import.import.span),
            ));
        }
        packages
            .entry(pending_import.import.path.clone())
            .or_insert(package);
    }
    Ok(())
}

fn collect_declarations(
    packages: &mut BTreeMap<String, Package>,
    source_packages: &BTreeMap<SourceId, String>,
    program: &Program,
    reject_duplicates: bool,
) -> Result<(), PackageError> {
    for declaration in &program.structs {
        let package = package_for_declaration(packages, source_packages, declaration.span)?;
        insert_declaration(
            &mut package.declarations,
            PackageDeclaration {
                name: declaration.name.clone(),
                kind: PackageDeclarationKind::Struct,
                type_params: declaration
                    .type_params
                    .iter()
                    .map(|param| param.name.clone())
                    .collect(),
                visibility: declaration.visibility,
                span: declaration.span,
            },
            &package.path,
            reject_duplicates,
        )?;
    }

    for declaration in &program.enums {
        let package = package_for_declaration(packages, source_packages, declaration.span)?;
        insert_declaration(
            &mut package.declarations,
            PackageDeclaration {
                name: declaration.name.clone(),
                kind: PackageDeclarationKind::Enum,
                type_params: declaration
                    .type_params
                    .iter()
                    .map(|param| param.name.clone())
                    .collect(),
                visibility: declaration.visibility,
                span: declaration.span,
            },
            &package.path,
            reject_duplicates,
        )?;
    }

    for declaration in &program.functions {
        let package = package_for_declaration(packages, source_packages, declaration.span)?;
        let package_path = package.path.clone();
        let package_declaration = PackageDeclaration {
            name: declaration.name.clone(),
            kind: if declaration.receiver.is_some() {
                PackageDeclarationKind::Method
            } else {
                PackageDeclarationKind::Function
            },
            type_params: declaration
                .type_params
                .iter()
                .map(|param| param.name.clone())
                .collect(),
            visibility: declaration.visibility,
            span: declaration.span,
        };

        if let Some(receiver) = &declaration.receiver {
            let methods = package.methods.entry(receiver.ty.name.clone()).or_default();
            insert_declaration(
                methods,
                package_declaration,
                &package_path,
                reject_duplicates,
            )?;
        } else {
            insert_declaration(
                &mut package.declarations,
                package_declaration,
                &package_path,
                reject_duplicates,
            )?;
        }
    }

    Ok(())
}

fn package_for_declaration<'a>(
    packages: &'a mut BTreeMap<String, Package>,
    source_packages: &BTreeMap<SourceId, String>,
    span: Span,
) -> Result<&'a mut Package, PackageError> {
    let package_path = source_packages.get(&span.source).ok_or_else(|| {
        PackageError::new(
            format!(
                "declaration source ID {} is not part of the package graph",
                span.source.index()
            ),
            Some(span),
        )
    })?;
    Ok(packages
        .get_mut(package_path)
        .expect("every source package path has a collected package"))
}

fn insert_declaration(
    declarations: &mut BTreeMap<String, PackageDeclaration>,
    declaration: PackageDeclaration,
    package_path: &str,
    reject_duplicates: bool,
) -> Result<(), PackageError> {
    if declarations.contains_key(&declaration.name) {
        if !reject_duplicates {
            return Ok(());
        }
        return Err(PackageError::new(
            format!(
                "duplicate declaration `{}` in package `{package_path}`",
                declaration.name
            ),
            Some(declaration.span),
        ));
    }
    declarations.insert(declaration.name.clone(), declaration);
    Ok(())
}

fn topological_order(packages: &BTreeMap<String, Package>) -> Result<Vec<String>, PackageError> {
    let mut states = BTreeMap::<String, VisitState>::new();
    let mut stack = Vec::new();
    let mut order = Vec::new();

    for path in packages.keys() {
        visit_package(path, packages, &mut states, &mut stack, &mut order)?;
    }

    Ok(order)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum VisitState {
    Visiting,
    Done,
}

fn visit_package(
    path: &str,
    packages: &BTreeMap<String, Package>,
    states: &mut BTreeMap<String, VisitState>,
    stack: &mut Vec<String>,
    order: &mut Vec<String>,
) -> Result<(), PackageError> {
    if states.get(path) == Some(&VisitState::Done) {
        return Ok(());
    }

    states.insert(path.to_string(), VisitState::Visiting);
    stack.push(path.to_string());
    let package = packages
        .get(path)
        .expect("topological traversal only visits collected packages");

    for import in &package.imports {
        match states.get(&import.path) {
            Some(VisitState::Visiting) => {
                let cycle_start = stack
                    .iter()
                    .position(|entry| entry == &import.path)
                    .expect("a visiting package is present in the traversal stack");
                let mut cycle = stack[cycle_start..].to_vec();
                cycle.push(import.path.clone());
                return Err(PackageError::new(
                    format!("package import cycle: {}", cycle.join(" -> ")),
                    Some(import.span),
                ));
            }
            Some(VisitState::Done) => {}
            None => visit_package(&import.path, packages, states, stack, order)?,
        }
    }

    stack.pop();
    states.insert(path.to_string(), VisitState::Done);
    order.push(path.to_string());
    Ok(())
}

fn is_identifier(value: &str) -> bool {
    let mut chars = value.chars();
    matches!(chars.next(), Some(first) if first.is_ascii_alphabetic() || first == '_')
        && chars.all(|ch| ch.is_ascii_alphanumeric() || ch == '_')
}

#[cfg(test)]
mod tests {
    use std::{
        fs,
        path::{Path, PathBuf},
        sync::atomic::{AtomicU64, Ordering},
    };

    use crate::{discover_project, load_source_files, parse_sources};

    use super::*;

    static NEXT_TEMP_ID: AtomicU64 = AtomicU64::new(0);

    struct TempProject {
        root: PathBuf,
    }

    impl TempProject {
        fn new(label: &str) -> Self {
            let id = NEXT_TEMP_ID.fetch_add(1, Ordering::Relaxed);
            let root = std::env::temp_dir().join(format!(
                "mallang-package-test-{}-{label}-{id}",
                std::process::id()
            ));
            fs::create_dir_all(&root).unwrap();
            let root = fs::canonicalize(root).unwrap();
            let project = Self { root };
            project.write("mallang.toml", "[project]\nname = \"hello\"\n");
            project
        }

        fn write(&self, path: &str, contents: &str) {
            let path = self.root.join(path);
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent).unwrap();
            }
            fs::write(path, contents).unwrap();
        }

        fn graph(&self) -> Result<PackageGraph, PackageError> {
            let project = discover_project(&self.root).unwrap();
            let loaded = load_source_files(project.source_files().iter().cloned()).unwrap();
            let program = parse_sources(&loaded.sources, &loaded.source_ids).unwrap();
            build_package_graph(&project, &loaded.sources, &program)
        }
    }

    impl Drop for TempProject {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.root);
        }
    }

    #[test]
    fn builds_packages_declarations_and_dependency_first_order() {
        let project = TempProject::new("valid");
        project.write(
            "src/main.mlg",
            "package main\nimport \"hello/greet\"\nfunc main() { greet.Print() }\n",
        );
        project.write(
            "src/greet/greet.mlg",
            "package greet\npub type Message struct {}\npub type State[T] enum { Empty Full(T) }\npub func Print() {}\n",
        );
        project.write(
            "src/greet/method.mlg",
            "package greet\npub func (con self Message) Show() {}\n",
        );

        let graph = project.graph().unwrap();

        assert_eq!(graph.packages().len(), 2);
        assert_eq!(graph.build_order(), &["hello/greet", "hello"]);
        let greet = graph.package("hello/greet").unwrap();
        assert_eq!(greet.source_ids.len(), 2);
        assert_eq!(
            greet.declarations["Message"].kind,
            PackageDeclarationKind::Struct
        );
        assert_eq!(greet.declarations["Print"].visibility, Visibility::Public);
        assert_eq!(
            greet.declarations["State"].kind,
            PackageDeclarationKind::Enum
        );
        assert_eq!(greet.declarations["State"].type_params, ["T"]);
        assert_eq!(
            greet.methods["Message"]["Show"].kind,
            PackageDeclarationKind::Method
        );
        let main = graph.package("hello").unwrap();
        assert_eq!(main.name, "main");
        assert_eq!(main.imports[0].qualifier, "greet");
    }

    #[test]
    fn requires_package_declarations_to_match_source_directories() {
        let missing = TempProject::new("missing-package");
        missing.write("src/main.mlg", "func main() {}\n");
        let missing_error = missing.graph().unwrap_err();
        assert!(missing_error
            .message
            .contains("must declare `package main`"));

        let mismatch = TempProject::new("mismatched-package");
        mismatch.write("src/main.mlg", "package main\nfunc main() {}\n");
        mismatch.write("src/greet/greet.mlg", "package other\nfunc Print() {}\n");
        let mismatch_error = mismatch.graph().unwrap_err();
        assert_eq!(
            mismatch_error.message,
            "package `other` does not match source directory package `greet`"
        );
    }

    #[test]
    fn rejects_unresolved_and_invalid_imports() {
        let unresolved = TempProject::new("unresolved");
        unresolved.write(
            "src/main.mlg",
            "package main\nimport \"hello/missing\"\nfunc main() {}\n",
        );
        let unresolved_error = unresolved.graph().unwrap_err();
        assert_eq!(
            unresolved_error.message,
            "unresolved import `hello/missing`"
        );

        let invalid = TempProject::new("invalid-import");
        invalid.write(
            "src/main.mlg",
            "package main\nimport \"hello/not-valid\"\nfunc main() {}\n",
        );
        let invalid_error = invalid.graph().unwrap_err();
        assert_eq!(
            invalid_error.message,
            "invalid import path `hello/not-valid`"
        );
    }

    #[test]
    fn resolves_compiler_owned_standard_packages() {
        let project = TempProject::new("standard-package");
        project.write(
            "src/main.mlg",
            "package main\nimport \"std/strings\"\nfunc main() {}\n",
        );

        let graph = project.graph().unwrap();
        let strings = graph.package("std/strings").unwrap();

        assert!(strings.source_ids.is_empty());
        assert_eq!(strings.name, "strings");
        assert_eq!(
            strings.declarations["byteLen"].kind,
            PackageDeclarationKind::Function
        );
        assert_eq!(graph.build_order(), &["std/strings", "hello"]);
    }

    #[test]
    fn rejects_duplicate_file_imports_and_qualifiers() {
        let duplicate = TempProject::new("duplicate-import");
        duplicate.write(
            "src/main.mlg",
            "package main\nimport \"hello/greet\"\nimport \"hello/greet\"\nfunc main() {}\n",
        );
        duplicate.write("src/greet/greet.mlg", "package greet\nfunc Print() {}\n");
        let duplicate_error = duplicate.graph().unwrap_err();
        assert_eq!(duplicate_error.message, "duplicate import `hello/greet`");

        let qualifier = TempProject::new("duplicate-qualifier");
        qualifier.write(
            "src/main.mlg",
            "package main\nimport \"hello/first/util\"\nimport \"hello/second/util\"\nfunc main() {}\n",
        );
        qualifier.write("src/first/util/util.mlg", "package util\nfunc A() {}\n");
        qualifier.write("src/second/util/util.mlg", "package util\nfunc B() {}\n");
        let qualifier_error = qualifier.graph().unwrap_err();
        assert!(qualifier_error.message.contains("same qualifier `util`"));
    }

    #[test]
    fn rejects_import_cycles_with_the_closing_import_span() {
        let project = TempProject::new("cycle");
        project.write(
            "src/main.mlg",
            "package main\nimport \"hello/a\"\nfunc main() {}\n",
        );
        project.write(
            "src/a/a.mlg",
            "package a\nimport \"hello/b\"\nfunc A() {}\n",
        );
        project.write(
            "src/b/b.mlg",
            "package b\nimport \"hello/a\"\nfunc B() {}\n",
        );

        let error = project.graph().unwrap_err();

        assert_eq!(
            error.message,
            "package import cycle: hello/a -> hello/b -> hello/a"
        );
        assert!(error.span.is_some());
    }

    #[test]
    fn allows_same_declaration_name_in_different_packages() {
        let project = TempProject::new("package-scopes");
        project.write("src/main.mlg", "package main\nfunc main() {}\n");
        project.write("src/a/a.mlg", "package a\npub func Open() {}\n");
        project.write("src/b/b.mlg", "package b\npub func Open() {}\n");

        let graph = project.graph().unwrap();

        assert!(graph
            .package("hello/a")
            .unwrap()
            .declarations
            .contains_key("Open"));
        assert!(graph
            .package("hello/b")
            .unwrap()
            .declarations
            .contains_key("Open"));
    }

    #[test]
    fn rejects_duplicate_declarations_within_a_package() {
        let project = TempProject::new("duplicate-declaration");
        project.write("src/main.mlg", "package main\nfunc main() {}\n");
        project.write("src/a/first.mlg", "package a\nfunc Open() {}\n");
        project.write("src/a/second.mlg", "package a\ntype Open struct {}\n");

        let error = project.graph().unwrap_err();

        assert_eq!(
            error.message,
            "duplicate declaration `Open` in package `hello/a`"
        );
    }

    #[test]
    fn maps_source_ids_back_to_their_package() {
        let project = TempProject::new("source-map");
        project.write("src/main.mlg", "package main\nfunc main() {}\n");
        project.write("src/greet/greet.mlg", "package greet\nfunc Print() {}\n");

        let discovered = discover_project(&project.root).unwrap();
        let loaded = load_source_files(discovered.source_files().iter().cloned()).unwrap();
        let program = parse_sources(&loaded.sources, &loaded.source_ids).unwrap();
        let graph = build_package_graph(&discovered, &loaded.sources, &program).unwrap();
        let greet_source = loaded
            .source_ids
            .iter()
            .copied()
            .find(|source| {
                loaded
                    .sources
                    .file(*source)
                    .unwrap()
                    .path()
                    .ends_with(Path::new("src/greet/greet.mlg"))
            })
            .unwrap();

        assert_eq!(
            graph.package_for_source(greet_source).unwrap().path,
            "hello/greet"
        );
    }
}
