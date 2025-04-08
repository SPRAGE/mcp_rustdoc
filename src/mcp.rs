//! Rust documentation fetcher MCP implementation.
//! 
//! This module provides functionality to fetch and cache Rust documentation from docs.rs.
//! It implements the MCP (Machine Control Protocol) server interface to expose documentation
//! fetching capabilities as a service.
//!
//! # Main Components
//! 
//! - [`DocFetcher`]: Main struct that handles fetching and caching of documentation
//! - [`DocsRsClient`]: Client for interacting with docs.rs API
//! - [`InMemoryCache`]: Cache implementation for storing fetched documentation
//!
//! # Example
//! ```no_run
//! use std::sync::Arc;
//! use rdoc_mcp::cache::InMemoryCache;
//! use rdoc_mcp::mcp::DocFetcher;
//! 
//! async fn example() {
//!     let cache = Arc::new(InMemoryCache::new("cache_dir".into()));
//!     let fetcher = DocFetcher::new(cache);
//!  
//! }
//! ```

use rmcp::model::{Implementation, ListPromptsResult, PaginatedRequestParam, ProtocolVersion, ServerCapabilities};
use rmcp::service::RequestContext;
use rmcp::{RoleServer, Error as McpError, ServerHandler, model::ServerInfo, tool};
use rmcp::{schemars, model::{IntoContents, Content}};
use std::sync::Arc;

use crate::cache::{Cache, InMemoryCache};
use crate::docs_parser::{DocsRsClient, DocsRsParams, DocContent, DocsFetchError};

/// Implements conversion from DocContent to MCP Contents.
impl IntoContents for DocContent {
    fn into_contents(self) -> Vec<Content> {
        vec![Content::text(self.content)]
    }
}

/// Implements conversion from DocsFetchError to MCP Contents.
impl IntoContents for DocsFetchError {
    fn into_contents(self) -> Vec<Content> {
        vec![Content::text(self.to_string())]
    }
}

/// Main struct responsible for fetching and caching Rust documentation.
/// 
/// `DocFetcher` provides functionality to fetch documentation from docs.rs
/// and caches the results in memory for faster subsequent access.
#[derive(Clone)]
pub struct DocFetcher {
    /// In-memory cache for storing fetched documentation
    cache: Arc<InMemoryCache>,
}

#[tool(tool_box)]
impl DocFetcher {
    /// Creates a new `DocFetcher` instance with the provided cache.
    ///
    /// # Arguments
    /// * `cache` - Arc-wrapped InMemoryCache instance for storing documentation
    pub fn new(cache: Arc<InMemoryCache>) -> Self {
        Self { cache }
    }

    /// Checks if a document with the given parameters exists in the cache.
    ///
    /// # Arguments
    /// * `params` - Parameters specifying the document to check for
    ///
    /// # Returns
    /// `true` if the document is cached, `false` otherwise
    #[allow(dead_code)]
    pub async fn is_cached(&self, params: &DocsRsParams) -> bool {
        self.cache.contains_key(params).await
    }

    /// Clears all cached documentation.
    #[allow(dead_code)]
    pub async fn clear_cache(&self) {
        self.cache.clear().await;
        tracing::info!("Document cache cleared.");
    }

    /// Fetches documentation for a Rust crate from docs.rs.
    ///
    /// This function will first check the cache for the requested documentation.
    /// If not found, it will fetch from docs.rs and cache the result.
    ///
    /// # Arguments
    /// * `crate_name` - Name of the crate to fetch documentation for
    /// * `version` - Version of the crate (e.g., "1.0.0")
    /// * `path` - Path to the specific documentation page
    ///
    /// # Returns
    /// * `Ok(DocContent)` - The fetched documentation content
    /// * `Err(DocsFetchError)` - If fetching fails
    #[tool(description = "Fetch Rust documentation from docs.rs")]
    async fn fetch_document(
        &self,
        #[tool(param)]
        #[schemars(description = "Name of the crate to fetch documentation for")]
        crate_name: String,

        #[tool(param)]
        #[schemars(description = "Version of crate, e.g. 1.0.0. If not specified, the latest version will be used.")]
        version: String,

        #[tool(param)]
        #[schemars(description = "Path to the specific documentation page (e.g., 'std/vec/struct.Vec.html'). If not specified, the document of the crate will be returned")]
        path: String,
    ) -> Result<DocContent, DocsFetchError> {
        let params = DocsRsParams {
            crate_name,
            version,
            path,
        };

        // Check cache first
        if let Some(cached_content) = self.cache.get(&params).await {
            tracing::info!("Cache hit for {:?}", params);
            return Ok(cached_content);
        }
        
        tracing::info!("Cache miss for {:?}. Fetching...", params);
        let client = DocsRsClient::new();
        match client.fetch_docs(params.clone()).await {
            Ok(doc_content) => {
                // Store in cache
                self.cache.insert(params, doc_content.clone()).await;
                Ok(doc_content)
            },
            Err(err) => Err(err),
        }
    }
}

#[tool(tool_box)]
impl ServerHandler for DocFetcher {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            protocol_version: ProtocolVersion::default(),
            capabilities: ServerCapabilities::builder()
                .enable_tools()  // We only need tools capability
                .build(),
            server_info: Implementation::from_build_env(),
            instructions: Some(
                "This server provides access to Rust documentation from docs.rs. \
                Use the 'fetch_document' tool to retrieve documentation for any crate. \
                Specify the crate name, version, and path to the documentation page you want to fetch. \
                Results are cached for better performance.".to_string()
            ),
        }
    }

    async fn list_prompts(
        &self,
        _request: PaginatedRequestParam,
        _: RequestContext<RoleServer>,
    ) -> Result<ListPromptsResult, McpError> {
        // We don't use prompts in this implementation
        Ok(ListPromptsResult {
            next_cursor: None,
            prompts: vec![],
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rmcp::model::{ClientCapabilities, ClientInfo, Implementation};
    use rmcp::{ServiceExt, model::CallToolRequestParam, transport::SseTransport};
    use rmcp::transport::sse_server::SseServer;
    use tempfile::tempdir;
    use std::time::Instant;
    use std::fs;

    fn setup_test_fetcher() -> (DocFetcher, Arc<InMemoryCache>) {
        let temp_dir = tempdir().unwrap();
        let cache_dir = temp_dir.path().to_path_buf();
        fs::create_dir_all(&cache_dir).unwrap(); 

        let cache = Arc::new(InMemoryCache::new(cache_dir));
        let fetcher = DocFetcher::new(cache.clone());
        (fetcher, cache)
    }

    #[tokio::test]
    async fn test_fetch_document() {
        let (doc_fetcher, _cache) = setup_test_fetcher();
        let result = doc_fetcher.fetch_document(
            "rand".to_string(),
            "0.9.0".to_string(),
            "rand/trait.Rng.html".to_string(),
        ).await.unwrap();

        assert!(!result.content.is_empty());
        assert!(result.content.contains("User-level interface for RNGs"));
    }
    
    #[tokio::test]
    async fn test_sse_server() {
        let addr = "127.0.0.1:8081"; // Let OS choose port
        let temp_cache_dir = tempdir().unwrap().path().to_path_buf();
        let server_cache = Arc::new(InMemoryCache::new(temp_cache_dir));

        let service_cache_clone = server_cache.clone();
        let server = SseServer::serve(addr.parse().unwrap()).await.unwrap();
        let port = server.config.bind.port();
        let ct = server.with_service(move || DocFetcher::new(service_cache_clone.clone()));
        
        let addr = format!("http://127.0.0.1:{}/sse", port);
        let transport = SseTransport::start(&addr).await.unwrap();

        let client_info = ClientInfo {
            protocol_version: Default::default(),
            capabilities: ClientCapabilities::default(),
            client_info: Implementation {
                name: "test sse client".to_string(),
                version: "0.0.1".to_string(),
            },
        };
        let client = client_info.serve(transport).await.unwrap();

        let result = client.call_tool(CallToolRequestParam {
            name: "fetch_document".into(),
            arguments: serde_json::json!({
                "crate_name": "rand",
                "version": "0.9.0",
                "path": "rand/trait.Rng.html",
            }).as_object().cloned(),
        }).await.unwrap();

        ct.cancel();

        assert!(!result.content.is_empty());
        assert!(result.content.iter().any(|c| c.as_text().unwrap().text.contains("User-level interface for RNGs")));
    }

    #[tokio::test]
    async fn test_cache_hit() {
        let (doc_fetcher, _) = setup_test_fetcher();
        let crate_name = "serde".to_string();
        let version = "1.0".to_string();
        let path = "serde/index.html".to_string();

        doc_fetcher.clear_cache().await;

        // First fetch - should be a cache miss
        println!("First fetch attempt (expect cache miss)...");
        let start1 = Instant::now();
        let result1 = doc_fetcher.fetch_document(
            crate_name.clone(),
            version.clone(),
            path.clone(),
        ).await.unwrap();
        let duration1 = start1.elapsed();
        println!("First fetch took: {:?}", duration1);

        // Second fetch - should be a cache hit
        println!("\nSecond fetch attempt (expect cache hit)...");
        let start2 = Instant::now();
        let result2 = doc_fetcher.fetch_document(
            crate_name.clone(),
            version.clone(),
            path.clone(),
        ).await.unwrap();
        let duration2 = start2.elapsed();
        println!("Second fetch took: {:?}", duration2);

        assert_eq!(result1.content, result2.content, "Cache returned different content");
        assert!(duration2 < duration1 / 10 || duration2.as_millis() < 10, 
                "Cache hit ({:?}) was not significantly faster than cache miss ({:?})", 
                duration2, duration1);
    }
} 