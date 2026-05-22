use std::path::{Path, PathBuf};
use thiserror::Error;
use tokio::fs;

/// Errors that can occur during file operations.
#[derive(Error, Debug)]
pub enum FileError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Path is not a file: {0}")]
    NotAFile(PathBuf),
    #[error("File not found: {0}")]
    NotFound(PathBuf),
}

/// Async file handle for reading and writing.
///
/// Uses atomic write-to-temp-then-rename to prevent data loss on crash.
pub struct FileHandle {
    path: PathBuf,
}

impl FileHandle {
    pub fn new(path: PathBuf) -> Self {
        Self { path }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Read the entire file asynchronously.
    pub async fn read_all(&self) -> Result<String, FileError> {
        let content = fs::read_to_string(&self.path).await?;
        Ok(content)
    }

    /// Atomically write content to disk.
    ///
    /// 1. Writes to a randomized temp file in the same directory.
    /// 2. Flushes to disk (`sync_data`).
    /// 3. Renames temp file → target path.
    pub async fn write_all(&self, content: &str) -> Result<(), FileError> {
        let dir = self
            .path
            .parent()
            .ok_or_else(|| FileError::NotAFile(self.path.clone()))?;
        let tmp_name = format!(
            ".{}.tmp-{:x}",
            self.path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("omni"),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos()
        );
        let tmp_path = dir.join(tmp_name);
        fs::write(&tmp_path, content).await?;
        fs::File::open(&tmp_path)
            .await?
            .sync_data()
            .await?;
        fs::rename(&tmp_path, &self.path).await?;
        // Best-effort sync of parent directory for crash durability.
        let _ = fs::File::open(dir).await?.sync_data().await;
        Ok(())
    }

    /// Check if the file exists on disk.
    pub async fn exists(&self) -> bool {
        fs::metadata(&self.path).await.is_ok()
    }
}

/// Central async file manager.
#[derive(Debug, Default)]
pub struct FileManager;

impl FileManager {
    pub fn new() -> Self {
        Self
    }

    pub async fn read_file(&self, path: &Path) -> Result<String, FileError> {
        let handle = FileHandle::new(path.to_path_buf());
        handle.read_all().await
    }

    pub async fn write_file(&self, path: &Path, content: &str) -> Result<(), FileError> {
        let handle = FileHandle::new(path.to_path_buf());
        handle.write_all(content).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn roundtrip() {
        let tmp = std::env::temp_dir().join("omni_test_roundtrip.txt");
        let fm = FileManager::new();
        fm.write_file(&tmp, "hello omni").await.unwrap();
        let content = fm.read_file(&tmp).await.unwrap();
        assert_eq!(content, "hello omni");
        fs::remove_file(&tmp).await.unwrap();
    }
}
