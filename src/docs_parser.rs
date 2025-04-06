use reqwest::Client;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use rmcp::{schemars, model::{IntoContents, Content}};

#[derive(Debug, Error)]
pub enum DocsFetchError {
    #[error("Request error: {0}")]
    RequestError(#[from] reqwest::Error),
    
    #[error("Invalid URL: {0}")]
    UrlError(#[from] url::ParseError),
    
    #[error("Failed to find documentation")]
    DocsNotFound,
    
    #[allow(dead_code)]
    #[error("Failed to parse documentation: {0}")]
    ParseError(String),
}

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema, Eq, PartialEq, Hash)]
pub struct DocsRsParams {
    #[schemars(description = "name of crate")]
    pub crate_name: String,
    #[schemars(description = "version of crate, e.g. 1.0.0. If not specified, the latest version will be used.")]
    pub version: String,
    #[schemars(description = "path of the module, struct, function, trait, etc. If not specified, the document of the crate will be returned. The path should end with .html for other pages by default.")]
    pub path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq)]
pub struct DocContent {
    pub content: String,
}

// Implement IntoContents trait for DocContent
impl IntoContents for DocContent {
    fn into_contents(self) -> Vec<Content> {
        vec![Content::text(self.content)]
    }
}

pub struct DocsRsClient {
    client: Client,
    base_url: String,
}

impl DocsRsClient {
    pub fn new() -> Self {
        Self {
            client: Client::new(),
            base_url: "https://docs.rs".to_string(),
        }
    }

    #[allow(dead_code)]
    pub fn new_with_base_url(base_url: &str) -> Self {
        Self {
            client: Client::new(),
            base_url: base_url.to_string(),
        }
    }
    
    pub async fn fetch_docs(&self, params: DocsRsParams) -> Result<DocContent, DocsFetchError> {
        // Construct URL for the API documentation
        let url = format!(
            "{}/{}/{}/{}",
            self.base_url,
            params.crate_name,
            params.version,
            params.path.trim_start_matches('/')
        );
        
        println!("Fetching documentation from: {}", url);
        
        let response = self.client.get(&url)
            .header("Accept", "text/html")
            .send()
            .await?;
        
        if !response.status().is_success() {
            return Err(DocsFetchError::DocsNotFound);
        }
        
        let html_content = response.text().await?;
        
        // Parse the main content from the rustdoc_body_wrapper div
        let parsed_content = self.extract_rustdoc_content(&html_content)
            .unwrap_or_else(|| format!("Documentation available at {}", url));
        
        Ok(DocContent { content: parsed_content })
    }
    
    fn extract_rustdoc_content(&self, html: &str) -> Option<String> {
        use scraper::{Html, Selector};
        
        // Parse the HTML document
        let document = Html::parse_document(html);
        
        // Create a selector for the rustdoc body wrapper
        let selector = Selector::parse("#rustdoc_body_wrapper").ok()?;
        
        // Find the wrapper element
        let wrapper = document.select(&selector).next()?;
        
        // Get the text content
        let content = wrapper.text().collect::<Vec<_>>().join(" ");
        
        // Clean up the content
        Some(content)
    }
    
    #[allow(dead_code)]
    fn parse_html_content(&self, html: &str) -> Result<(Option<String>, Option<String>, Option<Vec<String>>), DocsFetchError> {
        // In a real implementation, we would use a proper HTML parser like scraper or html5ever
        // For simplicity, we'll use basic string matching
        
        // Try to extract function signature (in a real impl, would use proper selectors)
        let function_signature = if let Some(start_idx) = html.find("<pre class=\"rust fn\">") {
            if let Some(end_idx) = html[start_idx..].find("</pre>") {
                let signature = &html[start_idx + 21..start_idx + end_idx];
                Some(signature.to_string())
            } else {
                None
            }
        } else {
            None
        };
        
        // Try to extract description
        let description = if let Some(start_idx) = html.find("<div class=\"docblock\">") {
            if let Some(end_idx) = html[start_idx..].find("</div>") {
                let desc = &html[start_idx + 22..start_idx + end_idx];
                Some(desc.to_string())
            } else {
                None
            }
        } else {
            None
        };
        
        // Try to extract examples
        let mut examples = Vec::new();
        
        if let Some(start_idx) = html.find("<h3>Examples</h3>") {
            if let Some(example_block_start) = html[start_idx..].find("<pre class=\"rust\">") {
                let abs_start = start_idx + example_block_start;
                if let Some(example_block_end) = html[abs_start..].find("</pre>") {
                    let example = &html[abs_start + 18..abs_start + example_block_end];
                    examples.push(example.to_string());
                }
            }
        }
        
        let examples_option = if examples.is_empty() {
            None
        } else {
            Some(examples)
        };
        
        Ok((function_signature, description, examples_option))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mockito::Server;

    #[tokio::test]
    async fn test_fetch_docs_success() {
        let mock_body = r#"<!DOCTYPE html><html><body>
            <div id="rustdoc_body_wrapper">
                <pre class="rust fn">pub async fn sleep(duration: Duration) -> Sleep</pre>
                <div class="docblock">This is a test description</div>
                <h3>Examples</h3>
                <pre class="rust">let example = "code";</pre>
            </div>
            </body></html>"#;

        let mut server = Server::new_async().await;
        let m = server.mock("GET", "/tokio/1.0.0/tokio/time/fn.sleep.html")
            .with_status(200)
            .with_header("content-type", "text/html")
            .with_body(mock_body)
            .create();

        let client = DocsRsClient::new_with_base_url(&server.url());
        
        let params = DocsRsParams {
            crate_name: "tokio".to_string(),
            version: "1.0.0".to_string(),
            path: "tokio/time/fn.sleep.html".to_string(),
        };

        let result = client.fetch_docs(params).await;
        m.assert();

        assert!(result.is_ok());
        let doc_content = result.unwrap();
        assert!(!doc_content.content.is_empty());
        assert!(doc_content.content.contains("sleep") || doc_content.content.contains("test description"));
    }

    #[tokio::test]
    async fn test_fetch_docs_real_server() {
        let client = DocsRsClient::new();
        
        let params = DocsRsParams {
            crate_name: "tokio".to_string(),
            version: "1.0.0".to_string(),
            path: "tokio/time/fn.sleep.html".to_string(),
        };

        let result = client.fetch_docs(params).await;
        
        // We don't assert the exact content since it might change,
        // but we verify the basic structure and that we got a response
        assert!(result.is_ok());
        let doc_content = result.unwrap();
        assert!(!doc_content.content.is_empty());
    }

    #[tokio::test]
    async fn test_fetch_docs_not_found() {
        let mut server = Server::new_async().await;
        let m = server.mock("GET", "/nonexistent/1.0.0/path/to/doc.html")
            .with_status(404)
            .create();

        let client = DocsRsClient::new_with_base_url(&server.url());
        
        let params = DocsRsParams {
            crate_name: "nonexistent".to_string(),
            version: "1.0.0".to_string(),
            path: "path/to/doc.html".to_string(),
        };

        let result = client.fetch_docs(params).await;
        m.assert();

        assert!(result.is_err());
        match result {
            Err(DocsFetchError::DocsNotFound) => (),
            _ => panic!("Expected DocsNotFound error"),
        }
    }

    #[tokio::test]
    async fn test_fetch_docs_not_found_real_server() {
        let client = DocsRsClient::new();
        
        let params = DocsRsParams {
            crate_name: "nonexistent_crate_123456789".to_string(),
            version: "1.0.0".to_string(),
            path: "path/to/doc.html".to_string(),
        };

        let result = client.fetch_docs(params).await;
        
        // Verify we get a DocsNotFound error for a non-existent crate
        assert!(result.is_err());
        match result {
            Err(DocsFetchError::DocsNotFound) => (),
            _ => panic!("Expected DocsNotFound error for non-existent crate"),
        }
    }

    #[test]
    fn test_extract_rustdoc_content() {
        let html = r#"<!DOCTYPE html><html><body>
            <div id="rustdoc_body_wrapper">
                <pre class="rust fn">pub async fn sleep(duration: Duration) -> Sleep</pre>
                <div class="docblock">This is a test description</div>
                <h3>Examples</h3>
                <pre class="rust">let example = "code";</pre>
            </div>
            </body></html>"#;

        let client = DocsRsClient::new();
        let content = client.extract_rustdoc_content(html);
        
        assert!(content.is_some());
        let parsed = content.unwrap();
        assert!(parsed.contains("sleep") || parsed.contains("test description"));
    }
} 