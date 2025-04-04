use serde_json::{json, Value};
use std::error::Error;
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // Using direct HTTP for testing
    http_client().await?;
    
    Ok(())
}

async fn http_client() -> Result<(), Box<dyn Error>> {
    // URL of the running MCP server
    let server_url = "http://127.0.0.1:8080";
    
    // Example: Query docs.rs for tokio::time::sleep function
    let query_tokio = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "mcp.fetch_document",
        "params": {
            "crate_name": "tokio",
            "version": "1.0.0",
            "path": "tokio/time/fn.sleep.html"
        }
    });
    
    println!("Querying tokio::time::sleep...");
    let response = query_mcp(server_url, query_tokio).await?;
    
    if let Some(error) = response.get("error") {
        println!("Error: {}", error);
    } else if let Some(result) = response.get("result") {
        // Get the content field from the DocContent response
        if let Some(content) = result.get("content") {
            let content_str = content.as_str().unwrap_or("No content");
            let preview = if content_str.len() > 200 {
                &content_str[0..200]
            } else {
                content_str
            };
            println!("Response preview: {}...", preview);
        } else {
            println!("No content field found in response");
        }
    }
    
    // Example: Query docs.rs for serde_json::to_string function
    let query_serde_json = json!({
        "jsonrpc": "2.0",
        "id": 2,
        "method": "mcp.fetch_document",
        "params": {
            "crate_name": "serde_json",
            "version": "1.0.0",
            "path": "serde_json/fn.to_string.html"
        }
    });
    
    println!("\nQuerying serde_json::to_string...");
    let response = query_mcp(server_url, query_serde_json).await?;
    
    if let Some(error) = response.get("error") {
        println!("Error: {}", error);
    } else if let Some(result) = response.get("result") {
        // Get the content field from the DocContent response
        if let Some(content) = result.get("content") {
            let content_str = content.as_str().unwrap_or("No content");
            let preview = if content_str.len() > 200 {
                &content_str[0..200]
            } else {
                content_str
            };
            println!("Response preview: {}...", preview);
        } else {
            println!("No content field found in response");
        }
    }
    
    Ok(())
}

async fn query_mcp(server_url: &str, query: Value) -> Result<Value, Box<dyn Error>> {
    let client = reqwest::Client::new();
    let response = client.post(server_url)
        .json(&query)
        .send()
        .await?
        .json::<Value>()
        .await?;
    
    Ok(response)
} 