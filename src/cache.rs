//! Cache management utilities
//!
//! This module provides a cache manager with file-based caching, supporting
//! version-based invalidation, file dependency tracking, and time-based expiration.
//!
//! ## Features
//!
//! - **Version keys**: Invalidate cache when version changes
//! - **File dependencies**: Invalidate when watched files change
//! - **Time-based expiration**: Invalidate after a duration
//! - **Serialization**: JSON-based storage with serde
//!
//! ## Examples
//!
//! ```rust,no_run
//! use xx::cache::CacheManager;
//! use std::time::Duration;
//!
//! // Create a cache manager
//! let cache = CacheManager::builder()
//!     .cache_dir("/tmp/my-cache")
//!     .version("1.0.0")
//!     .fresh_duration(Duration::from_secs(3600))
//!     .build()
//!     .unwrap();
//!
//! // Use the cache
//! let key = "my-data";
//! if let Some(data) = cache.get::<Vec<String>>(key) {
//!     println!("Cached: {:?}", data);
//! } else {
//!     let data = vec!["computed".to_string(), "data".to_string()];
//!     cache.set(key, &data).unwrap();
//! }
//! ```

use serde::{Deserialize, Serialize, de::DeserializeOwned};
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

use crate::{XXResult, file, hash::hash_to_str};

/// A cached entry with metadata
#[derive(Debug, Serialize, Deserialize)]
struct CacheEntry<T> {
    /// The cached data
    data: T,
    /// When this entry was created
    created_at: u64,
    /// Version key used when creating this entry
    version: String,
    /// Hash of watched files at creation time
    files_hash: Option<String>,
}

/// Builder for CacheManager
#[derive(Default)]
pub struct CacheManagerBuilder {
    cache_dir: Option<PathBuf>,
    version: String,
    fresh_duration: Option<Duration>,
    fresh_files: Vec<PathBuf>,
}

impl CacheManagerBuilder {
    /// Set the cache directory
    pub fn cache_dir<P: AsRef<Path>>(mut self, dir: P) -> Self {
        self.cache_dir = Some(dir.as_ref().to_path_buf());
        self
    }

    /// Set the version key for cache invalidation
    ///
    /// When the version changes, all cached data is considered stale.
    pub fn version<S: Into<String>>(mut self, version: S) -> Self {
        self.version = version.into();
        self
    }

    /// Set the freshness duration
    ///
    /// Cached data older than this duration is considered stale.
    pub fn fresh_duration(mut self, duration: Duration) -> Self {
        self.fresh_duration = Some(duration);
        self
    }

    /// Add a file to watch for changes
    ///
    /// When any watched file changes, cached data is considered stale.
    pub fn fresh_file<P: AsRef<Path>>(mut self, path: P) -> Self {
        self.fresh_files.push(path.as_ref().to_path_buf());
        self
    }

    /// Add multiple files to watch for changes
    pub fn fresh_files<I, P>(mut self, paths: I) -> Self
    where
        I: IntoIterator<Item = P>,
        P: AsRef<Path>,
    {
        for path in paths {
            self.fresh_files.push(path.as_ref().to_path_buf());
        }
        self
    }

    /// Build the CacheManager
    pub fn build(self) -> XXResult<CacheManager> {
        let cache_dir = self
            .cache_dir
            .ok_or_else(|| crate::error!("cache_dir is required"))?;

        file::mkdirp(&cache_dir)?;

        Ok(CacheManager {
            cache_dir,
            version: self.version,
            fresh_duration: self.fresh_duration,
            fresh_files: self.fresh_files,
        })
    }
}

/// A cache manager for file-based caching
pub struct CacheManager {
    cache_dir: PathBuf,
    version: String,
    fresh_duration: Option<Duration>,
    fresh_files: Vec<PathBuf>,
}

impl CacheManager {
    /// Create a new CacheManagerBuilder
    pub fn builder() -> CacheManagerBuilder {
        CacheManagerBuilder::default()
    }

    /// Get a value from the cache
    ///
    /// Returns None if:
    /// - The key doesn't exist
    /// - The cached data is stale (version mismatch, expired, files changed)
    /// - The data can't be deserialized
    pub fn get<T: DeserializeOwned>(&self, key: &str) -> Option<T> {
        let path = self.cache_path(key);

        if !path.exists() {
            return None;
        }

        let content = file::read_to_string(&path).ok()?;
        let entry: CacheEntry<T> = serde_json::from_str(&content).ok()?;

        if !self.is_entry_fresh(
            key,
            entry.created_at,
            &entry.version,
            entry.files_hash.as_deref(),
        ) {
            return None;
        }

        trace!("Cache hit: {}", key);
        Some(entry.data)
    }

    /// Store a value in the cache
    pub fn set<T: Serialize>(&self, key: &str, data: &T) -> XXResult<()> {
        let path = self.cache_path(key);

        let entry = CacheEntry {
            data,
            created_at: SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            version: self.version.clone(),
            files_hash: self.compute_files_hash(),
        };

        let content = serde_json::to_string_pretty(&entry)
            .map_err(|e| crate::error!("Failed to serialize cache entry: {}", e))?;

        file::write(&path, content)?;
        trace!("Cache set: {}", key);
        Ok(())
    }

    /// Remove a value from the cache
    pub fn remove(&self, key: &str) -> XXResult<()> {
        let path = self.cache_path(key);
        file::remove_file(&path)
    }

    /// Clear all cached data
    pub fn clear(&self) -> XXResult<()> {
        file::remove_dir_all(&self.cache_dir)?;
        file::mkdirp(&self.cache_dir)
    }

    /// Get or compute a value
    ///
    /// Returns the cached value if fresh, otherwise computes and caches the new value.
    pub fn get_or_try<T, F, E>(&self, key: &str, f: F) -> Result<T, E>
    where
        T: Serialize + DeserializeOwned,
        F: FnOnce() -> Result<T, E>,
        E: From<crate::XXError>,
    {
        if let Some(value) = self.get::<T>(key) {
            return Ok(value);
        }

        let value = f()?;
        self.set(key, &value)?;
        Ok(value)
    }

    /// Check if a key exists and is fresh
    pub fn contains(&self, key: &str) -> bool {
        let path = self.cache_path(key);
        if !path.exists() {
            return false;
        }

        // Read and check metadata without deserializing the data
        if let Ok(content) = file::read_to_string(&path) {
            if let Ok(entry) = serde_json::from_str::<CacheEntry<serde_json::Value>>(&content) {
                return self.is_entry_fresh(
                    key,
                    entry.created_at,
                    &entry.version,
                    entry.files_hash.as_deref(),
                );
            }
        }

        false
    }

    /// Get the path to a cache file
    fn cache_path(&self, key: &str) -> PathBuf {
        let hash = hash_to_str(&key);
        self.cache_dir.join(format!("{}.json", hash))
    }

    /// Check if a cache entry is still fresh
    fn is_entry_fresh(
        &self,
        key: &str,
        created_at: u64,
        version: &str,
        files_hash: Option<&str>,
    ) -> bool {
        // Check version
        if version != self.version {
            trace!("Cache miss (version mismatch): {}", key);
            return false;
        }

        // Check freshness duration
        if let Some(duration) = self.fresh_duration {
            let now = SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();

            if now - created_at >= duration.as_secs() {
                trace!("Cache miss (expired): {}", key);
                return false;
            }
        }

        // Check watched files
        if let Some(stored_hash) = files_hash {
            let current_hash = self.compute_files_hash();
            if current_hash.as_deref() != Some(stored_hash) {
                trace!("Cache miss (files changed): {}", key);
                return false;
            }
        }

        true
    }

    /// Compute a hash of the watched files' modification times
    ///
    /// Returns a hash that includes information about all watched files.
    /// If a file doesn't exist or can't be accessed, we include a marker
    /// for that in the hash so that deletion or creation of files also
    /// invalidates the cache.
    fn compute_files_hash(&self) -> Option<String> {
        if self.fresh_files.is_empty() {
            return None;
        }

        // Include file existence/modification state for each watched file
        // Using Option<u64> so that missing files are represented as None
        // and affect the hash differently than existing files
        let mtimes: Vec<Option<u64>> = self
            .fresh_files
            .iter()
            .map(|path| file::modified_time(path).ok().map(|m| m.as_secs()))
            .collect();

        Some(hash_to_str(&mtimes))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cache_basic() {
        let tmpdir = tempfile::tempdir().unwrap();
        let cache = CacheManager::builder()
            .cache_dir(tmpdir.path())
            .version("1.0")
            .build()
            .unwrap();

        // Initially empty
        assert!(cache.get::<String>("key1").is_none());

        // Set and get
        cache.set("key1", &"value1".to_string()).unwrap();
        assert_eq!(cache.get::<String>("key1"), Some("value1".to_string()));

        // Remove
        cache.remove("key1").unwrap();
        assert!(cache.get::<String>("key1").is_none());
    }

    #[test]
    fn test_cache_version_invalidation() {
        let tmpdir = tempfile::tempdir().unwrap();

        // Create cache with version 1
        let cache_v1 = CacheManager::builder()
            .cache_dir(tmpdir.path())
            .version("1.0")
            .build()
            .unwrap();

        cache_v1.set("key", &"value".to_string()).unwrap();
        assert_eq!(cache_v1.get::<String>("key"), Some("value".to_string()));

        // Create cache with version 2
        let cache_v2 = CacheManager::builder()
            .cache_dir(tmpdir.path())
            .version("2.0")
            .build()
            .unwrap();

        // Should not find the old cached value
        assert!(cache_v2.get::<String>("key").is_none());
    }

    #[test]
    fn test_cache_duration_expiration() {
        let tmpdir = tempfile::tempdir().unwrap();
        let cache = CacheManager::builder()
            .cache_dir(tmpdir.path())
            .version("1.0")
            .fresh_duration(Duration::from_secs(0)) // Immediately expire
            .build()
            .unwrap();

        cache.set("key", &"value".to_string()).unwrap();
        // Should be expired immediately
        std::thread::sleep(Duration::from_millis(10));
        assert!(cache.get::<String>("key").is_none());
    }

    #[test]
    fn test_cache_contains() {
        let tmpdir = tempfile::tempdir().unwrap();
        let cache = CacheManager::builder()
            .cache_dir(tmpdir.path())
            .version("1.0")
            .build()
            .unwrap();

        assert!(!cache.contains("key"));
        cache.set("key", &"value".to_string()).unwrap();
        assert!(cache.contains("key"));
    }

    #[test]
    fn test_cache_clear() {
        let tmpdir = tempfile::tempdir().unwrap();
        let cache = CacheManager::builder()
            .cache_dir(tmpdir.path())
            .version("1.0")
            .build()
            .unwrap();

        cache.set("key1", &"value1".to_string()).unwrap();
        cache.set("key2", &"value2".to_string()).unwrap();

        cache.clear().unwrap();

        assert!(!cache.contains("key1"));
        assert!(!cache.contains("key2"));
    }

    #[test]
    fn test_cache_complex_types() {
        let tmpdir = tempfile::tempdir().unwrap();
        let cache = CacheManager::builder()
            .cache_dir(tmpdir.path())
            .version("1.0")
            .build()
            .unwrap();

        #[derive(Debug, Serialize, Deserialize, PartialEq)]
        struct TestData {
            name: String,
            values: Vec<i32>,
        }

        let data = TestData {
            name: "test".to_string(),
            values: vec![1, 2, 3],
        };

        cache.set("complex", &data).unwrap();
        let retrieved: Option<TestData> = cache.get("complex");
        assert_eq!(retrieved, Some(data));
    }
}
