# Rust API Documentation MCP

This is a Model Context Protocol (MCP) implementation that allows querying Rust API documentation from [docs.rs](https://docs.rs). Built with the `rmcp` crate.

## Features

- Query documentation for a specific function in a crate with a specific version
- Returns raw HTML documentation with metadata
- Supports both SSE server and stdio server modes

## Usage

### Prerequisites

- Rust and Cargo installed
- An MCP client

### Running the Server

```bash
# Start SSE server (default) on the default address 127.0.0.1:8080
cargo run

# Start SSE server with custom address
cargo run -- --address 0.0.0.0:3000

# Start stdio server
cargo run -- --server-type stdio

# Show help
cargo run -- --help
```

### CLI Options

```
Options:
  -s, --server-type <SERVER_TYPE>  Type of server to run [default: sse] [possible values: sse, stdio]
  -a, --address <ADDRESS>          Address for the SSE server [default: 127.0.0.1:8080]
  -h, --help                       Print help
  -V, --version                    Print version
```

### Connecting to the Server

You can connect to the server using direct HTTP for simple testing. The example client in `examples/client.rs` demonstrates this approach.

### Testing

The project includes both unit tests and integration tests:

```bash
# Run all tests
cargo test

# Run unit tests only
cargo test --lib

# Run integration tests only
cargo test --test integration_test
```

Note: Integration tests start actual server instances on different ports, so ensure ports 8081 and 8082 are available on your system.

### API

The MCP exposes the following tool:

#### fetch_document

Parameters:
- `crate_name`: Name of the crate (e.g., "serde", "tokio")
- `version`: Version of the crate (e.g., "1.0.0", "latest")
- `path`: Path to the specific item you want documentation for (e.g., "serde/ser/trait.Serializer.html")

Example call with raw JSON-RPC:

```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "method": "mcp.fetch_document",
  "params": {
    "crate_name": "tokio",
    "version": "1.0.0",
    "path": "tokio/time/fn.sleep.html"
  }
}
```

Example response:

```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "result": "<!DOCTYPE html><html lang=\"en\">...[HTML content]..."
}
```

### Example Use Cases

#### Query the `std::fs::read_to_string` function

```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "method": "mcp.fetch_document",
  "params": {
    "crate_name": "std",
    "version": "latest",
    "path": "std/fs/fn.read_to_string.html"
  }
}
```

#### Query the `serde_json::to_string` function for version 1.0.0

```json
{
  "jsonrpc": "2.0",
  "id": 2,
  "method": "mcp.fetch_document",
  "params": {
    "crate_name": "serde_json",
    "version": "1.0.0", 
    "path": "serde_json/fn.to_string.html"
  }
}
```

## Future Improvements

- Parse the HTML content to extract function signatures, descriptions, and examples
- Support searching for functions across crates
- Add caching for frequently accessed documentation
- Implement automatic version detection for "latest" version requests
- Support for markdown rendering of documentation