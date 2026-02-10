//! Configuration path navigation.

use std::fmt;

/// A path to a configuration value.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ConfigPath {
    segments: Vec<PathSegment>,
}

/// A segment in a configuration path.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum PathSegment {
    /// A named key (e.g., "config").
    Key(String),
    /// An array index (e.g., [0]).
    Index(usize),
}

impl ConfigPath {
    /// Creates an empty (root) path.
    pub fn root() -> Self {
        ConfigPath {
            segments: Vec::new(),
        }
    }

    /// Creates a path from a single key.
    pub fn from_key(key: impl Into<String>) -> Self {
        ConfigPath {
            segments: vec![PathSegment::Key(key.into())],
        }
    }

    /// Parses a path from a string (e.g., "config.app.name" or "items[0].value").
    pub fn parse(path: &str) -> Result<Self, PathParseError> {
        if path.is_empty() {
            return Ok(Self::root());
        }

        let mut segments = Vec::new();
        let mut current = String::new();
        let mut chars = path.chars().peekable();

        while let Some(c) = chars.next() {
            match c {
                '.' => {
                    if !current.is_empty() {
                        segments.push(PathSegment::Key(current.clone()));
                        current.clear();
                    }
                }
                '[' => {
                    if !current.is_empty() {
                        segments.push(PathSegment::Key(current.clone()));
                        current.clear();
                    }
                    let mut index_str = String::new();
                    while let Some(&c) = chars.peek() {
                        if c == ']' {
                            chars.next();
                            break;
                        }
                        index_str.push(chars.next().unwrap());
                    }
                    let index: usize = index_str.parse().map_err(|_| PathParseError {
                        path: path.to_string(),
                        message: format!("Invalid index: {}", index_str),
                    })?;
                    segments.push(PathSegment::Index(index));
                }
                _ => {
                    current.push(c);
                }
            }
        }

        if !current.is_empty() {
            segments.push(PathSegment::Key(current));
        }

        Ok(ConfigPath { segments })
    }

    /// Returns the segments of this path.
    pub fn segments(&self) -> &[PathSegment] {
        &self.segments
    }

    /// Returns true if this is the root path.
    pub fn is_root(&self) -> bool {
        self.segments.is_empty()
    }

    /// Returns the depth of this path.
    pub fn depth(&self) -> usize {
        self.segments.len()
    }

    /// Appends a key to this path.
    pub fn push_key(&mut self, key: impl Into<String>) {
        self.segments.push(PathSegment::Key(key.into()));
    }

    /// Appends an index to this path.
    pub fn push_index(&mut self, index: usize) {
        self.segments.push(PathSegment::Index(index));
    }

    /// Creates a new path with a key appended.
    pub fn join_key(&self, key: impl Into<String>) -> Self {
        let mut new = self.clone();
        new.push_key(key);
        new
    }

    /// Creates a new path with an index appended.
    pub fn join_index(&self, index: usize) -> Self {
        let mut new = self.clone();
        new.push_index(index);
        new
    }

    /// Returns the parent path, or None if this is root.
    pub fn parent(&self) -> Option<Self> {
        if self.segments.is_empty() {
            None
        } else {
            let mut segments = self.segments.clone();
            segments.pop();
            Some(ConfigPath { segments })
        }
    }

    /// Returns the last segment, if any.
    pub fn last(&self) -> Option<&PathSegment> {
        self.segments.last()
    }
}

impl fmt::Display for ConfigPath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (i, segment) in self.segments.iter().enumerate() {
            match segment {
                PathSegment::Key(key) => {
                    if i > 0 {
                        write!(f, ".")?;
                    }
                    write!(f, "{}", key)?;
                }
                PathSegment::Index(index) => {
                    write!(f, "[{}]", index)?;
                }
            }
        }
        Ok(())
    }
}

impl fmt::Display for PathSegment {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PathSegment::Key(key) => write!(f, "{}", key),
            PathSegment::Index(index) => write!(f, "[{}]", index),
        }
    }
}

/// Error parsing a configuration path.
#[derive(Debug, Clone)]
pub struct PathParseError {
    pub path: String,
    pub message: String,
}

impl fmt::Display for PathParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Invalid path '{}': {}", self.path, self.message)
    }
}

impl std::error::Error for PathParseError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_path() {
        let path = ConfigPath::parse("config.app.name").unwrap();
        assert_eq!(path.depth(), 3);
        assert_eq!(path.to_string(), "config.app.name");
    }

    #[test]
    fn test_parse_path_with_index() {
        let path = ConfigPath::parse("items[0].value").unwrap();
        assert_eq!(path.depth(), 3);
        assert_eq!(path.to_string(), "items[0].value");
    }

    #[test]
    fn test_join_path() {
        let path = ConfigPath::root().join_key("config").join_key("app");
        assert_eq!(path.to_string(), "config.app");
    }

    #[test]
    fn test_parent_path() {
        let path = ConfigPath::parse("config.app.name").unwrap();
        let parent = path.parent().unwrap();
        assert_eq!(parent.to_string(), "config.app");
    }
}
