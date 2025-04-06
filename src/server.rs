use rmcp::model::{Implementation, ListPromptsResult, PaginatedRequestParam, Prompt, PromptArgument, ProtocolVersion, ServerCapabilities};
use rmcp::service::RequestContext;
use rmcp::{RoleServer, ServiceExt};
use rmcp::{Error as McpError, ServerHandler, model::ServerInfo, tool, transport::stdio};
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
    use rmcp::model::{ClientCapabilities, ClientInfo, Implementation};
    use rmcp::{ServiceExt, model::CallToolRequestParam, transport::SseTransport};

    #[tokio::test]
    async fn test_fetch_document() {
        let doc_fetcher = DocFetcher::new();
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
        let server = SseServer::serve(addr.parse().unwrap())
            .await
            .unwrap()
            .with_service(DocFetcher::new);

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
}
