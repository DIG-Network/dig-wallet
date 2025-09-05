use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::marker::PhantomData;
use crate::error::WalletError;

/// A simple file-based cache implementation similar to the TypeScript FileCache
pub struct FileCache<T> 
where
    T: Serialize + for<'de> Deserialize<'de>,
{
    cache_dir: PathBuf,
    _phantom: PhantomData<T>,
}

impl<T> FileCache<T> 
where
    T: Serialize + for<'de> Deserialize<'de>,
{
    /// Create a new FileCache instance
    pub fn new(relative_file_path: &str, base_dir: Option<&Path>) -> Result<Self, WalletError> {
        let base_path = match base_dir {
            Some(dir) => dir.to_path_buf(),
            None => dirs::home_dir()
                .ok_or_else(|| WalletError::FileSystemError("Could not find home directory".to_string()))?
                .join(".dig"),
        };
        
        let cache_dir = base_path.join(relative_file_path);
        
        let cache = Self { 
            cache_dir,
            _phantom: PhantomData,
        };
        cache.ensure_directory_exists()?;
        
        Ok(cache)
    }

    /// Ensure the cache directory exists
    fn ensure_directory_exists(&self) -> Result<(), WalletError> {
        if !self.cache_dir.exists() {
            fs::create_dir_all(&self.cache_dir)
                .map_err(|e| WalletError::FileSystemError(format!("Failed to create cache directory: {}", e)))?;
        }
        Ok(())
    }

    /// Get the cache file path for a given key
    fn get_cache_file_path(&self, key: &str) -> PathBuf {
        self.cache_dir.join(format!("{}.json", key))
    }

    /// Retrieve cached data by key
    pub fn get(&self, key: &str) -> Result<Option<T>, WalletError> {
        let cache_file_path = self.get_cache_file_path(key);
        
        if !cache_file_path.exists() {
            return Ok(None);
        }

        let raw_data = fs::read_to_string(&cache_file_path)
            .map_err(|e| WalletError::FileSystemError(format!("Failed to read cache file: {}", e)))?;
        
        let data: T = serde_json::from_str(&raw_data)
            .map_err(|e| WalletError::SerializationError(format!("Failed to deserialize cache data: {}", e)))?;
        
        Ok(Some(data))
    }

    /// Save data to the cache
    pub fn set(&self, key: &str, data: &T) -> Result<(), WalletError> {
        let cache_file_path = self.get_cache_file_path(key);
        
        let serialized_data = serde_json::to_string_pretty(data)
            .map_err(|e| WalletError::SerializationError(format!("Failed to serialize cache data: {}", e)))?;
        
        fs::write(&cache_file_path, serialized_data)
            .map_err(|e| WalletError::FileSystemError(format!("Failed to write cache file: {}", e)))?;
        
        Ok(())
    }

    /// Delete cached data by key
    pub fn delete(&self, key: &str) -> Result<(), WalletError> {
        let cache_file_path = self.get_cache_file_path(key);
        
        if cache_file_path.exists() {
            fs::remove_file(&cache_file_path)
                .map_err(|e| WalletError::FileSystemError(format!("Failed to delete cache file: {}", e)))?;
        }
        
        Ok(())
    }

    /// Retrieve all cached keys in the directory
    pub fn get_cached_keys(&self) -> Result<Vec<String>, WalletError> {
        if !self.cache_dir.exists() {
            return Ok(vec![]);
        }

        let entries = fs::read_dir(&self.cache_dir)
            .map_err(|e| WalletError::FileSystemError(format!("Failed to read cache directory: {}", e)))?;

        let mut keys = Vec::new();
        
        for entry in entries {
            let entry = entry
                .map_err(|e| WalletError::FileSystemError(format!("Failed to read directory entry: {}", e)))?;
            
            if let Some(file_name) = entry.file_name().to_str() {
                if file_name.ends_with(".json") {
                    let key = file_name.strip_suffix(".json").unwrap_or(file_name);
                    keys.push(key.to_string());
                }
            }
        }
        
        Ok(keys)
    }

    /// Clear all cached data
    pub fn clear(&self) -> Result<(), WalletError> {
        let keys = self.get_cached_keys()?;
        
        for key in keys {
            self.delete(&key)?;
        }
        
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReservedCoinCache {
    pub coin_id: String,
    pub expiry: u64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::{Deserialize, Serialize};
    use tempfile::TempDir;

    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
    struct TestData {
        value: String,
        number: i32,
    }

    #[test]
    fn test_file_cache_operations() {
        let temp_dir = TempDir::new().unwrap();
        let cache = FileCache::<TestData>::new("test_cache", Some(temp_dir.path())).unwrap();

        let test_data = TestData {
            value: "test".to_string(),
            number: 42,
        };

        // Test set and get
        cache.set("test_key", &test_data).unwrap();
        let retrieved = cache.get("test_key").unwrap().unwrap();
        assert_eq!(retrieved, test_data);

        // Test get non-existent key
        let non_existent = cache.get("non_existent").unwrap();
        assert!(non_existent.is_none());

        // Test get_cached_keys
        let keys = cache.get_cached_keys().unwrap();
        assert_eq!(keys, vec!["test_key"]);

        // Test delete
        cache.delete("test_key").unwrap();
        let deleted = cache.get("test_key").unwrap();
        assert!(deleted.is_none());
    }
}
