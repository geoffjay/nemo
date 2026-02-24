//! Source location tracking for configuration values.

use std::fmt;

/// A location in source code.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceLocation {
    /// The file path or name.
    pub file: String,
    /// The line number (1-indexed).
    pub line: usize,
    /// The column number (1-indexed).
    pub column: usize,
}

impl SourceLocation {
    /// Creates a new source location.
    pub fn new(file: impl Into<String>, line: usize, column: usize) -> Self {
        SourceLocation {
            file: file.into(),
            line,
            column,
        }
    }

    /// Creates a location for an unknown position.
    pub fn unknown() -> Self {
        SourceLocation {
            file: "<unknown>".to_string(),
            line: 0,
            column: 0,
        }
    }

    /// Returns true if this location is unknown.
    pub fn is_unknown(&self) -> bool {
        self.line == 0 && self.column == 0
    }

    /// Formats the location with context from source code.
    pub fn display_context(&self, source: &str, context_lines: usize) -> String {
        let lines: Vec<&str> = source.lines().collect();
        let mut output = String::new();

        if self.line == 0 || self.line > lines.len() {
            return output;
        }

        let start = self.line.saturating_sub(context_lines + 1);
        let end = (self.line + context_lines).min(lines.len());

        for (i, line) in lines.iter().enumerate().skip(start).take(end - start) {
            let line_num = i + 1;
            let prefix = if line_num == self.line { ">" } else { " " };
            output.push_str(&format!("{} {:4} | {}\n", prefix, line_num, line));

            if line_num == self.line && self.column > 0 {
                let padding = " ".repeat(self.column + 7);
                output.push_str(&format!("{}^\n", padding));
            }
        }

        output
    }
}

impl fmt::Display for SourceLocation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}:{}", self.file, self.line, self.column)
    }
}

impl Default for SourceLocation {
    fn default() -> Self {
        Self::unknown()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_location_display() {
        let loc = SourceLocation::new("config.xml", 10, 5);
        assert_eq!(loc.to_string(), "config.xml:10:5");
    }

    #[test]
    fn test_unknown_location() {
        let loc = SourceLocation::unknown();
        assert!(loc.is_unknown());
    }

    #[test]
    fn test_display_context() {
        let source = "line 1\nline 2\nline 3\nline 4\nline 5";
        let loc = SourceLocation::new("test.xml", 3, 2);
        let context = loc.display_context(source, 1);
        assert!(context.contains("line 2"));
        assert!(context.contains("line 3"));
        assert!(context.contains("line 4"));
    }
}
