use crate::buffer::TextBuffer;
use indexmap::IndexMap;
use std::sync::atomic::{AtomicU64, Ordering};

/// Opaque identifier for a tab.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct TabId(u64);

impl TabId {
    pub fn new() -> Self {
        static COUNTER: AtomicU64 = AtomicU64::new(1);
        Self(COUNTER.fetch_add(1, Ordering::Relaxed))
    }
}

impl Default for TabId {
    fn default() -> Self {
        Self::new()
    }
}

/// A single editor tab.
#[derive(Clone, Debug)]
pub struct Tab {
    pub id: TabId,
    /// Optional file path associated with this tab.
    pub path: Option<std::path::PathBuf>,
    pub buffer: TextBuffer,
    /// Cursor position as (line, col).
    pub cursor: (usize, usize),
    /// Viewport scroll offset in lines.
    pub scroll_line: usize,
}

impl Tab {
    pub fn new(path: Option<std::path::PathBuf>, text: &str) -> Self {
        Self {
            id: TabId::new(),
            path,
            buffer: TextBuffer::from_str(text),
            cursor: (0, 0),
            scroll_line: 0,
        }
    }

    pub fn is_dirty(&self) -> bool {
        self.buffer.is_dirty()
    }
}

/// Manages the set of open tabs with LRU ordering.
#[derive(Debug, Default)]
pub struct TabManager {
    tabs: IndexMap<TabId, Tab>,
    active: Option<TabId>,
}

impl TabManager {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn open(&mut self, path: Option<std::path::PathBuf>, text: &str) -> TabId {
        let tab = Tab::new(path, text);
        let id = tab.id;
        self.tabs.insert(id, tab);
        self.active = Some(id);
        id
    }

    pub fn close(&mut self, id: TabId) -> Option<Tab> {
        let removed = self.tabs.shift_remove(&id);
        if self.active == Some(id) {
            self.active = self.tabs.keys().next().copied();
        }
        removed
    }

    pub fn get(&self, id: TabId) -> Option<&Tab> {
        self.tabs.get(&id)
    }

    pub fn get_mut(&mut self, id: TabId) -> Option<&mut Tab> {
        self.tabs.get_mut(&id)
    }

    pub fn active(&self) -> Option<&Tab> {
        self.active.and_then(|id| self.tabs.get(&id))
    }

    pub fn active_mut(&mut self) -> Option<&mut Tab> {
        self.active.and_then(|id| self.tabs.get_mut(&id))
    }

    pub fn set_active(&mut self, id: TabId) {
        if self.tabs.contains_key(&id) {
            self.active = Some(id);
        }
    }

    pub fn iter(&self) -> indexmap::map::Values<'_, TabId, Tab> {
        self.tabs.values()
    }

    pub fn len(&self) -> usize {
        self.tabs.len()
    }

    pub fn is_empty(&self) -> bool {
        self.tabs.is_empty()
    }

    /// Close all tabs without prompting — used for emergency shutdown.
    pub fn clear(&mut self) {
        self.tabs.clear();
        self.active = None;
    }

    /// Returns IDs of tabs with unsaved changes.
    pub fn dirty_tab_ids(&self) -> Vec<TabId> {
        self.tabs
            .values()
            .filter(|t| t.is_dirty())
            .map(|t| t.id)
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn open_close_cycle() {
        let mut tm = TabManager::new();
        let id = tm.open(None, "hello");
        assert_eq!(tm.len(), 1);
        assert!(tm.close(id).is_some());
        assert!(tm.is_empty());
    }

    #[test]
    fn active_tracking() {
        let mut tm = TabManager::new();
        let id1 = tm.open(None, "a");
        let id2 = tm.open(None, "b");
        assert_eq!(tm.active().unwrap().id, id2);
        tm.set_active(id1);
        assert_eq!(tm.active().unwrap().id, id1);
    }

    #[test]
    fn dirty_detection() {
        let mut tm = TabManager::new();
        let id = tm.open(None, "x");
        tm.get_mut(id).unwrap().buffer.insert(1, "y");
        assert_eq!(tm.dirty_tab_ids(), vec![id]);
    }
}