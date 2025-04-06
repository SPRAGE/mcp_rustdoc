use crate::docs_parser::{DocContent, DocsRsParams};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, io};
use std::sync::Arc;
use tokio::fs;
use tokio::sync::RwLock;
use itertools::Itertools; // Added for grouping
use std::path::PathBuf;

/// Trait for a cache implementation.
#[async_trait]
pub trait Cache: Send + Sync {
    async fn get(&self, key: &DocsRsParams) -> Option<DocContent>;
    async fn insert(&self, key: DocsRsParams, value: DocContent);
    async fn contains_key(&self, key: &DocsRsParams) -> bool;
    async fn clear(&self);
    /// Save cache state to its configured directory.
    async fn save(&self) -> Result<(), io::Error>;
    /// Load cache state from its configured directory.
    async fn load(&self) -> Result<(), io::Error>;
}

/// Represents the data stored per crate file.
/// Key is the normalized string "{version}::{path}".
type CrateCacheData = HashMap<String, DocContent>;

// Keep DocsRsParams as the key for the in-memory representation
#[derive(Debug, Serialize, Deserialize, Default)]
struct CacheData {
    data: HashMap<DocsRsParams, DocContent>,
}

// Helper to normalize DocsRsParams (excluding crate_name) to a String key
fn normalize_key(params: &DocsRsParams) -> String {
    format!("{}::{}", params.version, params.path)
}

// Helper to denormalize a String key back to DocsRsParams
fn denormalize_key(crate_name: &str, normalized_key: &str) -> Result<DocsRsParams, String> {
    let parts: Vec<&str> = normalized_key.splitn(2, "::").collect();
    if parts.len() == 2 {
        Ok(DocsRsParams {
            crate_name: crate_name.to_string(),
            version: parts[0].to_string(),
            path: parts[1].to_string(),
        })
    } else {
        Err(format!("Invalid normalized key format: {}", normalized_key))
    }
}


#[derive(Debug, Clone)]
pub struct InMemoryCache {
    cache: Arc<RwLock<CacheData>>,
    cache_dir: PathBuf,
}

impl InMemoryCache {
    pub fn new(cache_dir: PathBuf) -> Self {
        Self {
            cache: Arc::new(RwLock::new(CacheData::default())),
            cache_dir,
        }
    }
}

#[async_trait]
impl Cache for InMemoryCache {
    async fn get(&self, key: &DocsRsParams) -> Option<DocContent> {
        self.cache.read().await.data.get(key).cloned()
    }

    async fn insert(&self, key: DocsRsParams, value: DocContent) {
        self.cache.write().await.data.insert(key, value);
    }

    async fn contains_key(&self, key: &DocsRsParams) -> bool {
        self.cache.read().await.data.contains_key(key)
    }

    async fn clear(&self) {
        self.cache.write().await.data.clear();
    }

    /// Saves the cache content to multiple JSON files within the configured directory.
    async fn save(&self) -> Result<(), io::Error> {
        let dir_path = &self.cache_dir;
        // 1. Prepare data outside the main async block to avoid holding lock across .await
        let data_to_save: HashMap<String, CrateCacheData> = { // New scope for the lock guard
            let cache_guard = self.cache.read().await;
            let data_map = &cache_guard.data;
    
            data_map.iter()
                .chunk_by(|(params, _)| &params.crate_name)
                .into_iter()
                .map(|(crate_name, group)| {
                    let crate_cache_data: CrateCacheData = group
                        .map(|(params, content)| (normalize_key(params), content.clone()))
                        .collect();
                    (crate_name.clone(), crate_cache_data)
                })
                .collect()
            // cache_guard is dropped here
        };
    
        // Ensure the main cache directory exists
        fs::create_dir_all(dir_path).await?;
    
        let mut saved_crate_files = std::collections::HashSet::new();
    
        // 2. Iterate over the prepared data (which is Send)
        for (crate_name, crate_cache_data) in &data_to_save {
            if crate_cache_data.is_empty() {
                continue;
            }
            let crate_file_name = format!("{}.json", crate_name);
            let crate_file_path = dir_path.join(&crate_file_name);
    
            let serialized = serde_json::to_string_pretty(crate_cache_data)
                .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
    
            // Perform async write
            fs::write(&crate_file_path, serialized).await?;
            saved_crate_files.insert(crate_file_path);
            tracing::debug!("Saved cache for crate '{}' to {:?}", crate_name, crate_file_name);
        }
    
        // Clean up stale files (this part is already async-safe)
        let mut entries = fs::read_dir(dir_path).await?;
        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            if path.is_file() && path.extension().map_or(false, |ext| ext == "json") {
                 if !saved_crate_files.contains(&path) {
                    match fs::remove_file(&path).await {
                        Ok(_) => tracing::info!("Removed stale cache file: {:?}", path),
                        Err(e) => tracing::warn!("Failed to remove stale cache file {:?}: {}", path, e),
                    }
                 }
            }
        }
    
         if data_to_save.is_empty() {
             tracing::info!("Cache is empty. Ensured cache directory {:?} is empty.", dir_path);
         }
    
        Ok(())
    }

     /// Loads cache content from multiple JSON files within the configured directory.
    async fn load(&self) -> Result<(), io::Error> {
        let dir_path = &self.cache_dir;
        if !dir_path.exists() {
            tracing::info!("Cache directory {:?} not found, starting with empty cache.", dir_path);
            // Ensure cache is empty
             *self.cache.write().await = CacheData::default();
            return Ok(());
        }
         if !dir_path.is_dir() {
            tracing::error!("Cache path {:?} is not a directory. Starting with empty cache.", dir_path);
             *self.cache.write().await = CacheData::default();
             // Return an error might be better here? For now, mimic old behavior.
            return Ok(());
        }


        let mut loaded_data = HashMap::new();
        let mut entries = fs::read_dir(dir_path).await?;
        let mut file_count = 0;
        let mut item_count = 0;

        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            if path.is_file() && path.extension().map_or(false, |ext| ext == "json") {
                 if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                     let crate_name = stem;
                     tracing::debug!("Attempting to load cache file for crate: {}", crate_name);
                     match fs::read_to_string(&path).await {
                         Ok(content) => {
                             if content.trim().is_empty() {
                                 tracing::warn!("Cache file {:?} is empty, skipping.", path);
                                 continue;
                             }
                             match serde_json::from_str::<CrateCacheData>(&content) {
                                 Ok(crate_cache_data) => {
                                     file_count += 1;
                                     for (norm_key, doc_content) in crate_cache_data {
                                         match denormalize_key(crate_name, &norm_key) {
                                             Ok(params) => {
                                                 loaded_data.insert(params, doc_content);
                                                 item_count += 1;
                                             }
                                             Err(e) => {
                                                 tracing::error!(
                                                    "Failed to denormalize key '{}' in file {:?}: {}. Skipping entry.",
                                                    norm_key, path, e
                                                 );
                                             }
                                         }
                                     }
                                 }
                                 Err(e) => {
                                     tracing::error!("Failed to deserialize cache file {:?}: {}. Skipping file.", path, e);
                                 }
                             }
                         }
                         Err(e) => {
                            tracing::error!("Failed to read cache file {:?}: {}. Skipping file.", path, e);
                         }
                     }
                 } else {
                     tracing::warn!("Skipping cache file with invalid name: {:?}", path);
                 }
            }
        }

        // Replace the current cache data with the loaded data
        let mut cache_guard = self.cache.write().await;
        cache_guard.data = loaded_data;

        tracing::info!(
            "Cache loaded from directory {:?} - {} files, {} items.",
            dir_path, file_count, item_count
        );

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn create_params(name: &str) -> DocsRsParams {
        DocsRsParams {
            crate_name: name.to_string(),
            version: "1.0".to_string(),
            path: name.to_string(),
        }
    }

    fn create_content(text: &str) -> DocContent {
        DocContent {
            content: text.to_string(),
        }
    }

    #[tokio::test]
    async fn test_insert_get_contains() {
        let dir = tempdir().unwrap();
        let cache: Arc<dyn Cache> = Arc::new(InMemoryCache::new(dir.path().to_path_buf()));
        let params1 = create_params("test1");
        let content1 = create_content("content1");
        let params2 = create_params("test2");

        assert!(!cache.contains_key(&params1).await);
        assert!(cache.get(&params1).await.is_none());

        cache.insert(params1.clone(), content1.clone()).await;

        assert!(cache.contains_key(&params1).await);
        assert_eq!(cache.get(&params1).await, Some(content1));
        assert!(!cache.contains_key(&params2).await);
    }

    #[tokio::test]
    async fn test_clear() {
        let dir = tempdir().unwrap();
        let cache: Arc<dyn Cache> = Arc::new(InMemoryCache::new(dir.path().to_path_buf()));
        let params1 = create_params("test1");
        let content1 = create_content("content1");

        cache.insert(params1.clone(), content1.clone()).await;
        assert!(cache.contains_key(&params1).await);

        cache.clear().await;
        assert!(!cache.contains_key(&params1).await);
        assert!(cache.get(&params1).await.is_none());
    }

    #[tokio::test]
    async fn test_save_load_single_crate() {
        let dir = tempdir().unwrap();
        let cache_dir_path = dir.path().to_path_buf();

        let cache1 = Arc::new(InMemoryCache::new(cache_dir_path.clone()));
        let params1 = create_params("serde"); // crate_name: "serde"
        let content1 = create_content("serde content");
        let params2 = DocsRsParams { // Same crate, different path/version
             crate_name: "serde".to_string(),
             version: "1.0.150".to_string(),
             path: "serde/derive".to_string(),
         };
        let content2 = create_content("serde derive content");

        cache1.insert(params1.clone(), content1.clone()).await;
        cache1.insert(params2.clone(), content2.clone()).await;

        // Save cache1
        cache1.save().await.expect("Failed to save cache");

        // Check if the specific file exists
        let crate_file = cache_dir_path.join("serde.json");
        assert!(crate_file.exists(), "Cache file for serde should exist");
         // Check file content (optional, but good for debugging)
         let file_content = fs::read_to_string(&crate_file).await.unwrap();
         println!("serde.json content: {}", file_content); // For inspection
         assert!(file_content.contains("1.0::serde"));
         assert!(file_content.contains("1.0.150::serde/derive"));
         assert!(file_content.contains("serde content"));
         assert!(file_content.contains("serde derive content"));

        // Create cache2 and load from directory
        let cache2 = Arc::new(InMemoryCache::new(cache_dir_path.clone()));
        cache2.load().await.expect("Failed to load cache");

        // Verify cache2 content
        assert_eq!(cache2.cache.read().await.data.len(), 2, "Cache should have 2 items");
        assert!(cache2.contains_key(&params1).await);
        assert_eq!(cache2.get(&params1).await, Some(content1));
        assert!(cache2.contains_key(&params2).await);
        assert_eq!(cache2.get(&params2).await, Some(content2));
    }

    #[tokio::test]
    async fn test_save_load_multiple_crates() {
        let dir = tempdir().unwrap();
        let cache_dir_path = dir.path().to_path_buf();

        let cache1 = Arc::new(InMemoryCache::new(cache_dir_path.clone()));
        let params_serde = create_params("serde");
        let content_serde = create_content("serde content");
        let params_tokio = create_params("tokio"); // crate_name: "tokio"
        let content_tokio = create_content("tokio content");
         let params_rand = DocsRsParams {
             crate_name: "rand".to_string(),
             version: "0.8".to_string(),
             path: "Rng".to_string(),
         };
         let content_rand = create_content("rand content");


        cache1.insert(params_serde.clone(), content_serde.clone()).await;
        cache1.insert(params_tokio.clone(), content_tokio.clone()).await;
        cache1.insert(params_rand.clone(), content_rand.clone()).await;

        // Save cache1
        cache1.save().await.expect("Failed to save cache");

        // Check if files exist
        assert!(cache_dir_path.join("serde.json").exists());
        assert!(cache_dir_path.join("tokio.json").exists());
        assert!(cache_dir_path.join("rand.json").exists());

        // Create cache2 and load
        let cache2 = Arc::new(InMemoryCache::new(cache_dir_path.clone()));
        cache2.load().await.expect("Failed to load cache");

        // Verify cache2 content
        assert_eq!(cache2.cache.read().await.data.len(), 3, "Cache should have 3 items");
        assert!(cache2.contains_key(&params_serde).await);
        assert_eq!(cache2.get(&params_serde).await, Some(content_serde));
        assert!(cache2.contains_key(&params_tokio).await);
        assert_eq!(cache2.get(&params_tokio).await, Some(content_tokio));
        assert!(cache2.contains_key(&params_rand).await);
        assert_eq!(cache2.get(&params_rand).await, Some(content_rand));
    }


    #[tokio::test]
    async fn test_save_removes_stale_files() {
        let dir = tempdir().unwrap();
        let cache_dir_path = dir.path().to_path_buf();

        let cache = Arc::new(InMemoryCache::new(cache_dir_path.clone()));
        let params_serde = create_params("serde");
        let content_serde = create_content("serde content");
        let params_tokio = create_params("tokio");
        let content_tokio = create_content("tokio content");

        // Save with two crates
        cache.insert(params_serde.clone(), content_serde.clone()).await;
        cache.insert(params_tokio.clone(), content_tokio.clone()).await;
        cache.save().await.expect("Initial save failed");
        assert!(cache_dir_path.join("serde.json").exists());
        assert!(cache_dir_path.join("tokio.json").exists());

        // Clear cache and add only serde back
        cache.clear().await;
        cache.insert(params_serde.clone(), content_serde.clone()).await;
        assert_eq!(cache.cache.read().await.data.len(), 1);

        // Save again
        cache.save().await.expect("Second save failed");

        // Verify tokio.json was removed
        assert!(cache_dir_path.join("serde.json").exists());
        assert!(!cache_dir_path.join("tokio.json").exists(), "tokio.json should have been removed");
    }


    #[tokio::test]
    async fn test_load_nonexistent_directory() {
        let dir = tempdir().unwrap();
        let cache_dir_path = dir.path().join("nonexistent_cache_dir"); // Does not exist, but path needed for new()
        
        let cache = Arc::new(InMemoryCache::new(cache_dir_path)); // Pass the path
        // Should not error, just start empty
        cache.load().await.expect("Loading non-existent dir failed"); // No argument

        assert!(cache.cache.read().await.data.is_empty(), "Cache should be empty");
    }

    #[tokio::test]
    async fn test_load_invalid_file_in_directory() {
         let dir = tempdir().unwrap();
         let cache_dir_path = dir.path().to_path_buf();
         let invalid_file_path = cache_dir_path.join("invalid.json");
         let valid_file_path = cache_dir_path.join("valid.json");

         // Create an invalid file
         fs::write(&invalid_file_path, "{invalid json}").await.unwrap();
         // Create a valid file
         let valid_params = create_params("valid");
         let valid_content = create_content("valid content");
         let valid_data: CrateCacheData = [(normalize_key(&valid_params), valid_content.clone())].into();
         fs::write(&valid_file_path, serde_json::to_string(&valid_data).unwrap()).await.unwrap();


        let cache = Arc::new(InMemoryCache::new(cache_dir_path.clone())); // Pass the path
        // Loading should succeed (log an error), but only load valid data
        cache.load().await.expect("Loading dir with invalid file failed"); // No argument

        assert_eq!(cache.cache.read().await.data.len(), 1, "Cache should contain one valid item");
        assert!(cache.contains_key(&valid_params).await, "Cache should contain the valid item");
         assert_eq!(cache.get(&valid_params).await, Some(valid_content));

    }

    #[tokio::test]
    async fn test_save_empty_cache_clears_dir() {
        let dir = tempdir().unwrap();
        let cache_dir_path = dir.path().to_path_buf();

        // Create some dummy files
        fs::write(cache_dir_path.join("stale1.json"), "{}").await.unwrap();
        fs::write(cache_dir_path.join("stale2.json"), "{}").await.unwrap();
        fs::write(cache_dir_path.join("not_json.txt"), "abc").await.unwrap();


        let cache: Arc<dyn Cache> = Arc::new(InMemoryCache::new(cache_dir_path.clone())); // Empty cache, needs path
        
        // Save empty cache
        cache.save().await.expect("Failed to save empty cache"); // No argument

        // Directory should exist, but json files should be gone
        assert!(cache_dir_path.exists());
        assert!(!cache_dir_path.join("stale1.json").exists(), "stale1.json should be removed");
        assert!(!cache_dir_path.join("stale2.json").exists(), "stale2.json should be removed");
        assert!(cache_dir_path.join("not_json.txt").exists(), "non-json file should remain");

    }

    #[tokio::test]
    async fn test_load_empty_file_in_directory() {
         let dir = tempdir().unwrap();
         let cache_dir_path = dir.path().to_path_buf();
         let empty_file_path = cache_dir_path.join("empty.json");
         fs::write(&empty_file_path, "").await.unwrap(); // Empty file

         let cache = Arc::new(InMemoryCache::new(cache_dir_path.clone())); // Needs path
         cache.load().await.expect("Loading dir with empty file failed"); // No argument

         assert!(cache.cache.read().await.data.is_empty(), "Cache should be empty after loading empty file");
    }
} 