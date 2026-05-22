//! Omni Core — foundational engine of the Omni IDE.
//!
//! Provides:
//! - `TextBuffer`: a rope-backed mutable text buffer with O(log n) edits and
//!   Unicode-aware line/column mapping.
//! - `TabManager`: multi-tab state with LRU ordering, change tracking, and
//!   async-friendly locking.
//! - `FileManager`: async file I/O with watcher integration, dirty-bit
//!   tracking, and atomic save via write-to-temp + rename.
//!
//! # Performance targets
//! - Insert/delete mid-file on a 10 MB buffer: < 1 ms.
//! - Open 1 000 tabs: < 50 MB resident memory.

pub mod buffer;
pub mod tabs;
pub mod file;

pub use buffer::TextBuffer;
pub use tabs::{Tab, TabId, TabManager};
pub use file::{FileHandle, FileManager};

/// Re-export of the rope type for downstream consumers.
pub use ropey::Rope;