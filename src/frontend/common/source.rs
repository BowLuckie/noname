use core::fmt;
use std::fmt::Write;
use std::path::PathBuf;

pub struct FileId {
    inner: u32,
}

/// a section of text based off a local offset. see [`SourceMap`] and [`SourceFile`]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Span {
    start: u32,
    end: u32,
}

#[derive(Debug, Clone)]
pub struct Spanned<T> {
    pub kind: T,
    pub span: Span,
}

#[derive(Default, Debug)]
pub struct SourceFile {
    pub name: PathBuf,
    pub text: Box<str>,
    pub base: u32,
    line_starts: Vec<u32>,
}

#[derive(Default, Debug)]
pub struct SourceMap {
    files: Vec<SourceFile>,
}

impl Span {
    pub fn start(&self) -> u32 {
        self.start
    }

    pub fn end(&self) -> u32 {
        self.end
    }

    /// generates a non-offsetted span. use Lexer::span to generate with appropriate offset
    pub(crate) fn new(start: u32, end: u32) -> Self {
        Self { start, end }
    }

    pub fn format(&self, sources: &SourceMap) -> Result<String, fmt::Error> {
        let mut buf = String::new();

        let (file, start_line, start_col) = sources.locate(*self);
        write!(
            buf,
            "{}:{}:{}",
            file.name.display(),
            start_line + 1,
            start_col + 1
        )?;

        Ok(buf)
    }
}

impl SourceMap {
    pub fn add(&mut self, name: PathBuf, text: String) -> FileId {
        let base = self
            .files
            .last()
            .map_or(0, |f| f.base + f.text.len() as u32 + 1); // leave 1 byte gap
        let mut line_starts = vec![0];
        for (i, b) in text.bytes().enumerate() {
            if b == b'\n' {
                line_starts.push(i as u32 + 1); // first byte of next line
            }
        }

        self.files.push(SourceFile {
            name,
            text: text.into(),
            base,
            line_starts,
        });
        FileId {
            inner: self.files.len() as u32 - 1,
        }
    }

    pub fn file(&self, id: FileId) -> &SourceFile {
        &self.files[id.inner as usize]
    }

    pub fn lookup(&self, global: u32) -> (&SourceFile, u32) {
        // last element of the first partition
        let i = self.files.partition_point(|f| f.base <= global) - 1;
        let file = &self.files[i];
        (file, global - file.base)
    }

    pub fn locate(&self, span: Span) -> (&SourceFile, usize, usize) {
        let (file, local) = self.lookup(span.start());
        let (line, col) = file.line_col(local);
        (file, line, col)
    }
}

impl SourceFile {
    pub fn line_col(&self, local: u32) -> (usize, usize) {
        let line = match self.line_starts.binary_search(&local) {
            Ok(i) => i,
            Err(i) => i - 1,
        };

        let start = self.line_starts[line] as usize;
        let col = self.text[start..local as usize].chars().count(); // count by chars not bytes
        (line, col) // 0 based 
    }

    pub fn line_text(&self, line: usize) -> &str {
        let start = self.line_starts[line] as usize;
        let end = self
            .line_starts
            .get(line + 1)
            .map_or(self.text.len(), |&num| num as usize);
        self.text[start..end].trim_end_matches(['\n', '\r'])
    }
}
