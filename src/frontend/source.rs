use std::path::PathBuf;

pub struct FileId {
    inner: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Span {
    start: u32,
    end: u32,
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
