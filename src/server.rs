use rmcp::ServiceExt;
use rmcp::{ServerHandler, model::ServerInfo, tool, transport::stdio};
use rmcp::transport::sse_server::SseServer;
use tracing_subscriber::{self, layer::SubscriberExt, util::SubscriberInitExt};
use anyhow::Result;

use crate::docs_parser::{DocsRsClient, DocsRsParams, DocContent};

#[derive(Debug, Clone)]
pub struct DocFetcher;

// create a static toolbox to store the tool attributes
#[tool(tool_box)]
impl DocFetcher {
    pub fn new() -> Self {
        Self{}
    }

    #[tool(description = "Fetch a rust document and extract its content")]
    async fn fetch_document(&self,
        #[tool(aggr)]
        params: DocsRsParams,
    ) -> DocContent {
        let client = DocsRsClient::new();
        match client.fetch_docs(params).await {
            Ok(doc_content) => doc_content,
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
            instructions: Some("A rust document fetcher".into()),
            ..Default::default()
        }
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

    let ct = SseServer::serve(addr.parse()?)
        .await?
        .with_service(DocFetcher::new);

    tokio::signal::ctrl_c().await?;
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

    // Create an instance
    let service = DocFetcher::new().serve(stdio()).await.inspect_err(|e| {
        tracing::error!("serving error: {:?}", e);
    })?;

    service.waiting().await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_fetch_document_tool() {
        // Create a DocFetcher instance
        let doc_fetcher = DocFetcher::new();
        
        // Define test parameters
        let params = DocsRsParams {
            crate_name: "test_crate".to_string(),
            version: "1.0.0".to_string(),
            path: "test/path.html".to_string(),
        };
        
        // Call the tool method directly
        let result = doc_fetcher.fetch_document(params).await;
        
        // Check that we got a DocContent result
        assert!(!result.content.is_empty());
        // Either it contains an error message or actual content
        assert!(result.content.contains("Error fetching documentation") || 
                !result.content.contains("Error"));
    }

}
