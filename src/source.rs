use std::{
    fs, io,
    path::{Path, PathBuf},
};

use crate::token::{SourceId, Span};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SourceLocation {
    pub line: usize,
    pub column: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceFile {
    id: SourceId,
    path: PathBuf,
    text: String,
    line_starts: Vec<usize>,
}

impl SourceFile {
    pub fn id(&self) -> SourceId {
        self.id
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn text(&self) -> &str {
        &self.text
    }

    pub fn location(&self, offset: usize) -> Option<SourceLocation> {
        if offset > self.text.len() || !self.text.is_char_boundary(offset) {
            return None;
        }

        let line_index = self.line_starts.partition_point(|start| *start <= offset) - 1;
        let line_start = self.line_starts[line_index];
        Some(SourceLocation {
            line: line_index + 1,
            column: self.text[line_start..offset].chars().count() + 1,
        })
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SourceMap {
    files: Vec<SourceFile>,
}

impl SourceMap {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_file(&mut self, path: impl Into<PathBuf>, text: impl Into<String>) -> SourceId {
        let id = SourceId::new(self.files.len());
        let text = text.into();
        let mut line_starts = vec![0];
        line_starts.extend(
            text.char_indices()
                .filter_map(|(index, ch)| (ch == '\n').then_some(index + 1)),
        );
        self.files.push(SourceFile {
            id,
            path: path.into(),
            text,
            line_starts,
        });
        id
    }

    pub fn file(&self, id: SourceId) -> Option<&SourceFile> {
        self.files.get(id.index())
    }

    pub fn format_diagnostic(&self, message: &str, span: Span) -> String {
        let Some(file) = self.file(span.source) else {
            return format!("{message} at {}..{}", span.start, span.end);
        };
        let Some(location) = file.location(span.start) else {
            return format!(
                "{}: {message} at {}..{}",
                file.path().display(),
                span.start,
                span.end
            );
        };

        format!(
            "{}:{}:{}: {message}",
            file.path().display(),
            location.line,
            location.column
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceSet {
    pub sources: SourceMap,
    pub source_ids: Vec<SourceId>,
}

#[derive(Debug)]
pub struct SourceLoadError {
    path: PathBuf,
    source: io::Error,
}

impl SourceLoadError {
    pub fn path(&self) -> &Path {
        &self.path
    }
}

impl std::fmt::Display for SourceLoadError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            formatter,
            "{}: failed to read source: {}",
            self.path.display(),
            self.source
        )
    }
}

impl std::error::Error for SourceLoadError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(&self.source)
    }
}

pub fn load_source_files<I, P>(paths: I) -> Result<SourceSet, SourceLoadError>
where
    I: IntoIterator<Item = P>,
    P: Into<PathBuf>,
{
    let mut sources = SourceMap::new();
    let mut source_ids = Vec::new();

    for path in paths {
        let path = path.into();
        let text = fs::read_to_string(&path).map_err(|source| SourceLoadError {
            path: path.clone(),
            source,
        })?;
        source_ids.push(sources.add_file(path, text));
    }

    Ok(SourceSet {
        sources,
        source_ids,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn distinguishes_locations_from_multiple_files() {
        let mut sources = SourceMap::new();
        let first = sources.add_file("src/main.mlg", "func main() {}\n");
        let second = sources.add_file(
            "src/greet/greet.mlg",
            "package greet\npub func Print() {}\n",
        );

        assert_ne!(first, second);
        assert_eq!(
            sources
                .file(second)
                .and_then(|file| file.location("package greet\n".len() + 4)),
            Some(SourceLocation { line: 2, column: 5 })
        );
    }

    #[test]
    fn formats_diagnostic_with_its_source_path_and_location() {
        let mut sources = SourceMap::new();
        sources.add_file("src/main.mlg", "func main() {}\n");
        let greet = sources.add_file(
            "src/greet/greet.mlg",
            "package greet\npub func Print() {}\n",
        );
        let start = "package greet\n".len() + 4;

        assert_eq!(
            sources.format_diagnostic("unexpected declaration", Span::new(greet, start, start + 4)),
            "src/greet/greet.mlg:2:5: unexpected declaration"
        );
    }

    #[test]
    fn counts_unicode_columns_as_characters() {
        let mut sources = SourceMap::new();
        let source = sources.add_file("unicode.mlg", "// 가나다\nfunc main() {}\n");
        let offset = "// 가나다\nfunc ".len();

        assert_eq!(
            sources.file(source).and_then(|file| file.location(offset)),
            Some(SourceLocation { line: 2, column: 6 })
        );
    }

    #[test]
    fn loads_explicit_source_files_in_caller_order() {
        let loaded = load_source_files(["examples/first.mlg", "examples/hello.mlg"]).unwrap();

        assert_eq!(loaded.source_ids.len(), 2);
        assert_eq!(
            loaded
                .sources
                .file(loaded.source_ids[0])
                .map(SourceFile::path),
            Some(Path::new("examples/first.mlg"))
        );
        assert_eq!(
            loaded
                .sources
                .file(loaded.source_ids[1])
                .map(SourceFile::path),
            Some(Path::new("examples/hello.mlg"))
        );
    }

    #[test]
    fn reports_the_source_path_that_failed_to_load() {
        let error = load_source_files(["examples/missing-source.mlg"]).unwrap_err();

        assert_eq!(error.path(), Path::new("examples/missing-source.mlg"));
        assert!(error.to_string().contains("failed to read source"));
    }
}
