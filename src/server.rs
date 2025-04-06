use rmcp::model::{Implementation, ListPromptsResult, PaginatedRequestParam, Prompt, PromptArgument, ProtocolVersion, ServerCapabilities};
use rmcp::service::RequestContext;
use rmcp::{RoleServer, ServiceExt};
use rmcp::{Error as McpError, ServerHandler, model::ServerInfo, tool, transport::stdio};
use rmcp::transport::sse_server::SseServer;
use tracing_subscriber::{self, layer::SubscriberExt, util::SubscriberInitExt};
use anyhow::Result;
use std::sync::Arc;
use std::path::PathBuf;
use std::time::Instant;

use crate::cache::{Cache, InMemoryCache};
use crate::docs_parser::{DocsRsClient, DocsRsParams, DocContent};

// const CACHE_FILE: &str = "./doc_cache.json"; // Remove single file constant
const CACHE_DIR: &str = ".cache"; // Define cache directory constant

#[derive(Clone)]
pub struct DocFetcher {
    cache: Arc<InMemoryCache>,
}

// create a static toolbox to store the tool attributes
#[tool(tool_box)]
impl DocFetcher {
    pub fn new(cache: Arc<InMemoryCache>) -> Self {
        Self { cache }
    }

    /// Checks if the document for the given parameters is already in the cache.
    pub async fn is_cached(&self, params: &DocsRsParams) -> bool {
        self.cache.contains_key(params).await
    }

    /// Clears the entire document cache.
    pub async fn clear_cache(&self) {
        self.cache.clear().await;
        tracing::info!("Document cache cleared.");
    }

    #[tool(description = "Fetch a rust document and extract its content")]
    async fn fetch_document(&self,
        #[tool(aggr)]
        params: DocsRsParams,
    ) -> DocContent {
        // Check cache first
        if let Some(cached_content) = self.cache.get(&params).await {
            tracing::info!("Cache hit for {:?}", params);
            return cached_content;
        }
        
        tracing::info!("Cache miss for {:?}. Fetching...", params);
        let client = DocsRsClient::new();
        match client.fetch_docs(params.clone()).await {
            Ok(doc_content) => {
                // Store in cache
                self.cache.insert(params, doc_content.clone()).await;
                doc_content
            },
            Err(err) => DocContent { 
                content: format!("Error fetching documentation: {}", err) 
            },
        }
    }
}

// impl call_tool and list_tool by querying static toolbox
#[tool(tool_box)]
impl ServerHandler for DocFetcher {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            protocol_version: ProtocolVersion::default(),
            capabilities: ServerCapabilities::builder()
                .enable_prompts()
                .enable_resources()
                .enable_tools()
                .build(),
            server_info: Implementation::from_build_env(),
            instructions: Some("This server provides a counter tool that can increment and decrement values. The counter starts at 0 and can be modified using the 'increment' and 'decrement' tools. Use 'get_value' to check the current count.".to_string()),
        }
    }

    async fn list_prompts(
        &self,
        _request: PaginatedRequestParam,
        _: RequestContext<RoleServer>,
    ) -> Result<ListPromptsResult, McpError> {
        Ok(ListPromptsResult {
            next_cursor: None,
            prompts: vec![Prompt::new(
                "example_prompt",
                Some("This is an example prompt that takes one required agrument, message"),
                Some(vec![PromptArgument {
                    name: "message".to_string(),
                    description: Some("A message to put in the prompt".to_string()),
                    required: Some(true),
                }]),
            )],
        })
    }
}

// start sse server
pub async fn start_sse_server(addr: &str) -> anyhow::Result<()> {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "debug".to_string().into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let cache_dir_path = PathBuf::from(CACHE_DIR);
    let cache = Arc::new(InMemoryCache::new());
    if let Err(e) = cache.load(&cache_dir_path).await {
        tracing::error!("Failed to load cache from {:?}: {}. Starting fresh.", cache_dir_path, e);
    }

    let server_cache = cache.clone(); // Clone Arc for the server service
    let ct = SseServer::serve(addr.parse()?) 
        .await?
        .with_service(move || DocFetcher::new(server_cache.clone()));

    tokio::signal::ctrl_c().await?;
    tracing::info!("Shutdown signal received. Saving cache...");
    if let Err(e) = cache.save(&cache_dir_path).await {
        tracing::error!("Failed to save cache to {:?}: {}", cache_dir_path, e);
    }
    ct.cancel();
    Ok(())
}

// start stdio server
pub async fn start_stdio_server() -> anyhow::Result<()> {
    // Initialize the tracing subscriber with file and stdout logging
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env().add_directive(tracing::Level::DEBUG.into()))
        .with_writer(std::io::stderr)
        .with_ansi(false)
        .init();

    tracing::info!("Starting MCP server");

    let cache_dir_path = PathBuf::from(CACHE_DIR);
    let cache = Arc::new(InMemoryCache::new());
    if let Err(e) = cache.load(&cache_dir_path).await {
        tracing::error!("Failed to load cache from {:?}: {}. Starting fresh.", cache_dir_path, e);
    }

    // Create an instance
    let service_cache = cache.clone(); // Clone Arc for the service
    let service = DocFetcher::new(service_cache).serve(stdio()).await.inspect_err(|e| {
        tracing::error!("serving error: {:?}", e);
    })?;

    service.waiting().await?;

    tracing::info!("Service finished. Saving cache...");
    if let Err(e) = cache.save(&cache_dir_path).await {
        tracing::error!("Failed to save cache to {:?}: {}", cache_dir_path, e);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use rmcp::model::{ClientCapabilities, ClientInfo, Implementation};
    use rmcp::{ServiceExt, model::CallToolRequestParam, transport::SseTransport};
    use tempfile::tempdir;

    fn setup_test_fetcher() -> (DocFetcher, Arc<InMemoryCache>) {
        let cache = Arc::new(InMemoryCache::new());
        let fetcher = DocFetcher::new(cache.clone());
        (fetcher, cache)
    }

    #[tokio::test]
    async fn test_fetch_document() {
        let (doc_fetcher, _cache) = setup_test_fetcher();
        let params = DocsRsParams {
            crate_name: "rand".to_string(),
            version: "0.9.0".to_string(),
            path: "rand/trait.Rng.html".to_string(),
        };

        let result = doc_fetcher.fetch_document(params).await;

        // println!("Tool result: {result:#?}");
        assert!(!result.content.is_empty());
        assert!(result.content.contains("User-level interface for RNGs"));
    }
    
    #[tokio::test]
    async fn test_sse_server() {
        // start sse server
        let addr = "127.0.0.1:8000";
        let temp_dir = tempdir().unwrap(); // Create temp dir for cache file
        let cache_path = temp_dir.path().join("sse_test_cache.json");
        let cache = Arc::new(InMemoryCache::new());
        // No need to load/save in this specific test, focus is on MCP comms

        let server_cache = cache.clone();
        let server = SseServer::serve(addr.parse().unwrap())
            .await
            .unwrap()
            .with_service(move || DocFetcher::new(server_cache.clone()));

        let transport = SseTransport::start(&format!("http://{}/sse", addr)).await.unwrap();

        let client_info = ClientInfo {
            protocol_version: Default::default(),
            capabilities: ClientCapabilities::default(),
            client_info: Implementation {
                name: "test sse client".to_string(),
                version: "0.0.1".to_string(),
            },
        };
        let client = client_info.serve(transport).await.inspect_err(|e| {
            println!("client error: {:?}", e);
        }).unwrap();

        let server_info = client.peer_info();
        println!("Connected to server: {server_info:#?}");

        let tools = client.list_tools(Default::default()).await.unwrap();
        println!("Available tools: {tools:#?}");

        let result = client.call_tool(CallToolRequestParam {
            name: "fetch_document".into(),
            arguments: serde_json::json!({
                "crate_name": "rand".to_string(),
                "version": "0.9.0".to_string(),
                "path": "rand".to_string(),
            }).as_object().cloned(),
        }).await.unwrap();

        server.cancel();

        // println!("Tool result: {result:#?}");
        assert!(!result.content.is_empty());
        assert!(result.content.iter().any(|c| c.as_text().unwrap().text.contains("Utilities for random number generation")));
    }

    #[tokio::test]
    async fn test_cache_hit() {
        // Basic tracing setup for observing cache logs during test execution
        let _ = tracing_subscriber::fmt().with_test_writer().try_init();

        let (doc_fetcher, cache) = setup_test_fetcher();
        let params = DocsRsParams {
            crate_name: "serde".to_string(),
            version: "1.0".to_string(), // Use a common crate/version
            path: "serde".to_string(),
        };

        // Clear the cache
        doc_fetcher.clear_cache().await;

        // Assert not cached initially
        assert!(!cache.contains_key(&params).await, "Document should not be cached initially");

        // First call - should be a cache miss
        println!("First fetch attempt (expect cache miss)...");
        let start1 = Instant::now();
        let result1 = doc_fetcher.fetch_document(params.clone()).await;
        let duration1 = start1.elapsed();
        println!("First fetch took: {:?}", duration1);
        assert!(!result1.content.is_empty(), "First fetch failed or returned empty content");
        assert!(!result1.content.contains("Error fetching documentation"), "First fetch resulted in an error: {}", result1.content);

        // Assert cached now
        assert!(cache.contains_key(&params).await, "Document should be cached after first fetch");

        // Second call - should be a cache hit
        println!("\nSecond fetch attempt (expect cache hit)...");
        let start2 = Instant::now();
        let result2 = doc_fetcher.fetch_document(params.clone()).await;
        let duration2 = start2.elapsed();
        println!("Second fetch took: {:?}", duration2);
        assert!(!result2.content.is_empty(), "Second fetch failed or returned empty content");

        // Verify results are the same
        assert_eq!(result1.content, result2.content, "Cache returned different content");

        // Verify cache was faster (allow for some small overhead)
        // Ensure duration2 is significantly less than duration1 (e.g., < 1/10th)
        // Add a small tolerance to avoid flakes due to tiny timings
        assert!(duration2 < duration1 / 10 || duration2.as_millis() < 10, 
                "Cache hit ({:?}) was not significantly faster than cache miss ({:?})", 
                duration2, duration1);

        println!("\nCache timing test complete. Second fetch was significantly faster.");
    }

    #[tokio::test]
    async fn test_clear_cache() {
        let _ = tracing_subscriber::fmt().with_test_writer().try_init();

        let (doc_fetcher, cache) = setup_test_fetcher();
        let params = DocsRsParams {
            crate_name: "tokio".to_string(), // Use a different crate for variety
            version: "1.30.0".to_string(),
            path: "tokio/fs".to_string(),
        };

        // Fetch document to populate cache
        println!("Fetching document to populate cache...");
        let _ = doc_fetcher.fetch_document(params.clone()).await;

        // Assert it's cached
        assert!(cache.contains_key(&params).await, "Document should be in cache directly after fetching");
        assert!(doc_fetcher.is_cached(&params).await, "Document should be cached after fetching (via fetcher)");
        println!("Document is cached.");

        // Clear the cache
        println!("Clearing cache...");
        doc_fetcher.clear_cache().await;

        // Assert it's no longer cached
        assert!(!cache.contains_key(&params).await, "Document should not be in cache directly after clearing");
        assert!(!doc_fetcher.is_cached(&params).await, "Document should not be cached after clearing (via fetcher)");
        println!("Cache successfully cleared.");
    }
}
