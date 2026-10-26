//! Source positions and human-readable errors.

use std::fmt;

/// A 1-based line and column in the source text.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
pub struct Pos {
    pub line: usize,
    pub col: usize,
}

impl Pos {
    pub const fn start() -> Self {
        Self { line: 1, col: 1 }
    }
}

impl fmt::Display for Pos {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}", self.line, self.col)
    }
}

/// One problem found in a document, with an optional fix hint.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Diagnostic {
    pub pos: Pos,
    pub message: String,
    pub hint: Option<String>,
}

impl Diagnostic {
    pub fn new(pos: Pos, message: impl Into<String>) -> Self {
        Self {
            pos,
            message: message.into(),
            hint: None,
        }
    }

    pub fn with_hint(mut self, hint: impl Into<String>) -> Self {
        self.hint = Some(hint.into());
        self
    }
}

impl fmt::Display for Diagnostic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.pos, self.message)?;
        if let Some(hint) = &self.hint {
            write!(f, "; {hint}")?;
        }
        Ok(())
    }
}

/// All problems found while reading a document, in source order.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Diagnostics(pub Vec<Diagnostic>);

impl Diagnostics {
    /// Renders one `file:line:col: message` line per diagnostic.
    pub fn render(&self, file: &str) -> String {
        self.0
            .iter()
            .map(|d| format!("{file}:{d}"))
            .collect::<Vec<_>>()
            .join("\n")
    }
}

impl fmt::Display for Diagnostics {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (i, d) in self.0.iter().enumerate() {
            if i > 0 {
                writeln!(f)?;
            }
            write!(f, "{d}")?;
        }
        Ok(())
    }
}

impl std::error::Error for Diagnostics {}
