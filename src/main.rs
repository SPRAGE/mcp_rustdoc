mod docs_parser;
mod server;
mod cache;
mod mcp;

use clap::{Parser, ValueEnum};
use anyhow::Result;

#[derive(Parser, Debug)]
#[command(version, about = "Rust API Documentation MCP Server")]
struct Cli {
    /// Type of server to run
    #[arg(short, long, value_enum, default_value_t = ServerType::Sse)]
    server_type: ServerType,

    /// Address for the SSE server
    #[arg(short, long, default_value = "127.0.0.1:8080")]
    address: String,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, ValueEnum)]
enum ServerType {
    /// Start an SSE server
    Sse,
    /// Start a stdio server
    Stdio,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    match cli.server_type {
        ServerType::Sse => {
            println!("Starting SSE server on {}", cli.address);
            server::start_sse_server(&cli.address).await?;
        },
        ServerType::Stdio => {
            server::start_stdio_server().await?;
        },
    }

    Ok(())
} 