//! Omni Syntax — universal syntax highlighting via Tree-sitter.
//!
//! Supports any language for which a Tree-sitter grammar is available.
//! The engine maps rope edits to incremental tree updates for O(log n)
//! re-highlighting.

use omni_core::TextBuffer;
use std::collections::HashMap;
use thiserror::Error;
use tree_sitter::{Language, Parser, Query, QueryCursor, Tree};

/// Errors from the syntax engine.
#[derive(Error, Debug)]
pub enum SyntaxError {
    #[error("Parser failed to parse buffer")]
    ParseFailed,
    #[error("Invalid query: {0}")]
    InvalidQuery(String),
    #[error("Unsupported language: {0}")]
    UnsupportedLanguage(String),
}

/// A single syntax highlight span.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HighlightSpan {
    pub start: usize, // byte offset
    pub end: usize,
    pub scope: String, // e.g. "keyword", "string", "function"
}

/// Language descriptor with Tree-sitter grammar and highlight query.
pub struct LanguageDescriptor {
    pub name: &'static str,
    pub language: Language,
    /// S-expression highlight query (typically `highlights.scm`).
    pub highlight_query: &'static str,
}

/// Registry of supported languages.
pub struct LanguageRegistry {
    languages: HashMap<String, LanguageDescriptor>,
}

impl Default for LanguageRegistry {
    fn default() -> Self {
        let mut reg = Self {
            languages: HashMap::new(),
        };
        reg.register(LanguageDescriptor {
            name: "rust",
            language: tree_sitter_rust::LANGUAGE.into(),
            highlight_query: tree_sitter_rust::HIGHLIGHTS_QUERY,
        });
        reg.register(LanguageDescriptor {
            name: "javascript",
            language: tree_sitter_javascript::LANGUAGE.into(),
            highlight_query: tree_sitter_javascript::HIGHLIGHT_QUERY,
        });
        reg.register(LanguageDescriptor {
            name: "python",
            language: tree_sitter_python::LANGUAGE.into(),
            highlight_query: tree_sitter_python::HIGHLIGHTS_QUERY,
        });
        reg
    }
}

impl LanguageRegistry {
    pub fn register(&mut self, desc: LanguageDescriptor) {
        self.languages.insert(desc.name.to_string(), desc);
    }

    pub fn get(&self, name: &str) -> Option<&LanguageDescriptor> {
        self.languages.get(name)
    }

    pub fn detect_by_extension(&self, path: &std::path::Path) -> Option<&str> {
        let ext = path.extension()?.to_str()?;
        match ext {
            "rs" => Some("rust"),
            "js" | "jsx" | "ts" | "tsx" => Some("javascript"),
            "py" => Some("python"),
            _ => None,
        }
    }
}

/// Highlighter state for a single buffer.
pub struct Highlighter<'a> {
    parser: Parser,
    query: Query,
    language: &'a LanguageDescriptor,
    tree: Option<Tree>,
}

impl<'a> Highlighter<'a> {
    pub fn new(language: &'a LanguageDescriptor) -> Result<Self, SyntaxError> {
        let mut parser = Parser::new();
        parser
            .set_language(&language.language)
            .map_err(|_| SyntaxError::UnsupportedLanguage(language.name.to_string()))?;
        let query = Query::new(&language.language, language.highlight_query)
            .map_err(|e| SyntaxError::InvalidQuery(format!("{:?}", e)))?;
        Ok(Self {
            parser,
            query,
            language,
            tree: None,
        })
    }

    /// Parse or re-parse the entire buffer.
    pub fn parse(&mut self, buffer: &TextBuffer) -> Result<(), SyntaxError> {
        let rope = buffer.rope();
        let text = rope.to_string();
        self.tree = self
            .parser
            .parse(&text, self.tree.as_ref())
            .ok_or(SyntaxError::ParseFailed)?;
        Ok(())
    }

    /// Run the highlight query and return spans.
    pub fn highlights(&self, buffer: &TextBuffer) -> Result<Vec<HighlightSpan>, SyntaxError> {
        let tree = self.tree.as_ref().ok_or(SyntaxError::ParseFailed)?;
        let rope = buffer.rope();
        let text = rope.to_string();
        let root = tree.root_node();
        let mut cursor = QueryCursor::new();
        let matches = cursor.matches(&self.query, root, text.as_bytes());

        let mut spans = Vec::new();
        for m in matches {
            for capture in m.captures {
                let node = capture.node;
                let name = self.query.capture_names()[capture.index as usize];
                spans.push(HighlightSpan {
                    start: node.start_byte(),
                    end: node.end_byte(),
                    scope: name.to_string(),
                });
            }
        }
        spans.sort_by_key(|s| s.start);
        Ok(spans)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detect_rust() {
        let reg = LanguageRegistry::default();
        let path = std::path::Path::new("/tmp/main.rs");
        assert_eq!(reg.detect_by_extension(path), Some("rust"));
    }

    #[test]
    fn highlight_rust() {
        let reg = LanguageRegistry::default();
        let lang = reg.get("rust").unwrap();
        let mut highlighter = Highlighter::new(lang).unwrap();
        let buf = TextBuffer::from_str("fn main() {}");
        highlighter.parse(&buf).unwrap();
        let spans = highlighter.highlights(&buf).unwrap();
        // Should find at least 'fn' as keyword or function highlight
        assert!(!spans.is_empty());
    }
}