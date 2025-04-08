use rmcp::ServiceExt;
use rmcp::transport::{stdio, sse_server::SseServer};
use tracing_subscriber::{self, layer::SubscriberExt, util::SubscriberInitExt};
use std::sync::Arc;
use std::path::PathBuf;

use crate::cache::{InMemoryCache, Cache};
use crate::mcp::DocFetcher;

const CACHE_DIR: &str = ".cache";

pub async fn start_sse_server(addr: &str) -> anyhow::Result<()> {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "debug".to_string().into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let cache_dir_path = PathBuf::from(CACHE_DIR);
    let cache = Arc::new(InMemoryCache::new(cache_dir_path.clone()));
    if let Err(e) = cache.load().await {
        tracing::error!("Failed to load cache from {:?}: {}. Starting fresh.", cache_dir_path, e);
    }

    let server_cache = cache.clone();
    let ct = SseServer::serve(addr.parse()?) 
        .await?
        .with_service(move || DocFetcher::new(server_cache.clone()));

    tokio::signal::ctrl_c().await?;
    tracing::info!("Shutdown signal received. Saving cache...");
    if let Err(e) = cache.save().await {
        tracing::error!("Failed to save cache to {:?}: {}", cache_dir_path, e);
    }
    ct.cancel();
    Ok(())
}

pub async fn start_stdio_server() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env().add_directive(tracing::Level::DEBUG.into()))
        .with_writer(std::io::stderr)
        .with_ansi(false)
        .init();

    tracing::info!("Starting MCP server");

    let cache_dir_path = PathBuf::from(CACHE_DIR);
    let cache = Arc::new(InMemoryCache::new(cache_dir_path.clone()));
    if let Err(e) = cache.load().await {
        tracing::error!("Failed to load cache from {:?}: {}. Starting fresh.", cache_dir_path, e);
    }

    let service_cache = cache.clone();
    let service = DocFetcher::new(service_cache).serve(stdio()).await.inspect_err(|e| {
        tracing::error!("serving error: {:?}", e);
    })?;

    service.waiting().await?;

    tracing::info!("Service finished. Saving cache...");
    if let Err(e) = cache.save().await {
        tracing::error!("Failed to save cache to {:?}: {}", cache_dir_path, e);
    }
    Ok(())
}
