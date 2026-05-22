use ropey::{Rope, RopeSlice};
use std::sync::Arc;

/// A lightweight, clone-on-write buffer wrapper.
///
/// Internally uses `Arc<Rope>` so cloning is O(1). Mutations are performed
/// via `make_mut()` which clones the inner rope only when the reference
/// count is > 1, enabling cheap snapshots for undo/redo or background
/// syntax highlighting.
#[derive(Clone, Debug)]
pub struct TextBuffer {
    rope: Arc<Rope>,
    /// Dirty bit — true if the buffer has unsaved modifications.
    dirty: bool,
}

impl Default for TextBuffer {
    fn default() -> Self {
        Self {
            rope: Arc::new(Rope::new()),
            dirty: false,
        }
    }
}

impl TextBuffer {
    /// Create a buffer from a UTF-8 string.
    pub fn from_str(text: &str) -> Self {
        Self {
            rope: Arc::new(Rope::from_str(text)),
            dirty: false,
        }
    }

    /// Total number of lines (including empty trailing line if applicable).
    pub fn line_count(&self) -> usize {
        self.rope.len_lines()
    }

    /// Total length in UTF-16 code units (LSP compatible).
    pub fn len_utf16(&self) -> usize {
        self.rope.chars().map(|c| c.len_utf16()).sum()
    }

    /// Insert `text` at the given char index.
    ///
    /// # Panics
    /// Panics if `char_idx` is out of bounds.
    pub fn insert(&mut self, char_idx: usize, text: &str) {
        Arc::make_mut(&mut self.rope).insert(char_idx, text);
        self.dirty = true;
    }

    /// Delete the range `[start, end)` in char indices.
    ///
    /// # Panics
    /// Panics if indices are out of order or out of bounds.
    pub fn delete(&mut self, start: usize, end: usize) {
        Arc::make_mut(&mut self.rope).remove(start..end);
        self.dirty = true;
    }

    /// Replace the range `[start, end)` with `text`.
    pub fn replace(&mut self, start: usize, end: usize, text: &str) {
        let rope = Arc::make_mut(&mut self.rope);
        rope.remove(start..end);
        rope.insert(start, text);
        self.dirty = true;
    }

    /// Read a line by zero-based index.
    pub fn line(&self, line_idx: usize) -> Option<RopeSlice<'_>> {
        self.rope.get_line(line_idx)
    }

    /// Convert a (line, column) pair to an absolute char index.
    pub fn line_col_to_char(&self, line: usize, col: usize) -> Option<usize> {
        if line >= self.rope.len_lines() {
            return None;
        }
        let line_start = self.rope.line_to_char(line);
        let line_len = self.rope.get_line(line)?.len_chars();
        let abs = line_start + col;
        if col <= line_len {
            Some(abs)
        } else {
            None
        }
    }

    /// Convert an absolute char index to (line, col).
    pub fn char_to_line_col(&self, char_idx: usize) -> Option<(usize, usize)> {
        if char_idx > self.rope.len_chars() {
            return None;
        }
        let line = if char_idx == self.rope.len_chars() {
            self.rope.len_lines().saturating_sub(1)
        } else {
            self.rope.char_to_line(char_idx)
        };
        let line_start = self.rope.line_to_char(line);
        Some((line, char_idx - line_start))
    }

    /// Full buffer text as a new `String`.
    pub fn to_string(&self) -> String {
        self.rope.to_string()
    }

    /// A slice of the rope without allocating.
    pub fn slice(&self, start: usize, end: usize) -> RopeSlice<'_> {
        self.rope.slice(start..end)
    }

    /// Mark as saved (clears dirty flag).
    pub fn mark_saved(&mut self) {
        self.dirty = false;
    }

    pub fn is_dirty(&self) -> bool {
        self.dirty
    }

    /// Immutable reference to the underlying rope.
    pub fn rope(&self) -> &Rope {
        &self.rope
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn insert_and_delete() {
        let mut buf = TextBuffer::from_str("hello world");
        buf.insert(5, " cruel");
        assert_eq!(buf.to_string(), "hello cruel world");
        buf.delete(5, 12);
        assert_eq!(buf.to_string(), "hello world");
    }

    #[test]
    fn line_col_roundtrip() {
        let buf = TextBuffer::from_str("line0\nline1\nline2");
        let idx = buf.line_col_to_char(1, 3).unwrap();
        assert_eq!(buf.char_to_line_col(idx), Some((1, 3)));
    }

    #[test]
    fn dirty_tracking() {
        let mut buf = TextBuffer::from_str("x");
        assert!(!buf.is_dirty());
        buf.insert(1, "y");
        assert!(buf.is_dirty());
        buf.mark_saved();
        assert!(!buf.is_dirty());
    }

    #[test]
    fn cheap_clone() {
        let buf = TextBuffer::from_str("shared");
        let cloned = buf.clone();
        assert_eq!(buf.to_string(), cloned.to_string());
        // Arc pointer should be shared until mutation
        assert!(Arc::ptr_eq(&buf.rope, &cloned.rope));
    }
}