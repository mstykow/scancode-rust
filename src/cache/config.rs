use std::fs;
use std::io;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CacheConfig {
    root_dir: PathBuf,
}

impl CacheConfig {
    pub fn new(root_dir: PathBuf) -> Self {
        Self { root_dir }
    }

    pub fn from_scan_root(scan_root: &Path) -> Self {
        Self::new(scan_root.join(".scancode-cache"))
    }

    pub fn root_dir(&self) -> &Path {
        &self.root_dir
    }

    pub fn index_dir(&self) -> PathBuf {
        self.root_dir.join("index")
    }

    pub fn scan_results_dir(&self) -> PathBuf {
        self.root_dir.join("scan-results")
    }

    pub fn ensure_dirs(&self) -> io::Result<()> {
        fs::create_dir_all(self.index_dir())?;
        fs::create_dir_all(self.scan_results_dir())?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use tempfile::TempDir;

    use super::*;

    #[test]
    fn test_from_scan_root_uses_expected_directory_name() {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let config = CacheConfig::from_scan_root(temp_dir.path());
        assert_eq!(config.root_dir(), temp_dir.path().join(".scancode-cache"));
    }

    #[test]
    fn test_ensure_dirs_creates_expected_tree() {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let config = CacheConfig::from_scan_root(temp_dir.path());

        config
            .ensure_dirs()
            .expect("Failed to create cache directories");

        assert!(config.root_dir().exists());
        assert!(config.index_dir().exists());
        assert!(config.scan_results_dir().exists());
    }
}
