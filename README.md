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

## Configuration for MCP Clients (e.g., Cursor)

To connect an MCP client like Cursor to this server, you can use the following configuration.

### Stdio Server Configuration

If you are running the server in stdio mode, use a configuration similar to this:

```json
"rustdoc-mcp": {
  "command": "rdoc-mcp",
  "args": [
      "--server-type",
      "stdio"
  ]
}
```